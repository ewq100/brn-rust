# S2 shell and navigation — evidence

Branch `work/ux-s2-shell`, rebased onto `origin/work/ux-rebuild-20261105` at `23dfb4bd` (S1 appearance merged). Status: implemented and verified (automated); native-observed on synthetic data at the sizes below. Not owner-accepted, not merged.

## Commits

| Commit | Content |
|---|---|
| `999c6af7` | Shell model, sidebar/rail, workspace header, assistant pane and sheet, centred launcher, keymap, test migration |
| `c64e4397` | Quiet selected rows (`active` role), rail header inset, sheet state cleared while docked (+ unit test) |
| this file | Evidence |

## Shell model (`threads_layout.rs`)

`TileKind` stays the content-kind enum, so every body (`tile_body`) renders unchanged. `Layout` is now one value holding route and preferences, ready for the lead to persist:

- `primary`: the view filling the workspace; `Settings` is a primary view that returns to `before_settings`.
- Supporting pane: `support` kind (Chat = the assistant, or Linked/Compare/Connections), `support_open` preference, `support_width` (320, clamped 280–400). A Note hosts any supporting kind; a conversation hosts Linked/Compare/Connections, never a second chat.
- Sidebar: `sidebar_expanded` (224 / 56 rail), `sidebar_width` (208–280), `sidebar_deliberate` (owner expanded it on purpose).
- Session only: `sheet_open` (compact sheet showing), `focus` (Sidebar / Primary / Assistant).

Navigation rules: `open(kind)` (behind `show_tile`) replaces the primary view, or joins a supporting kind to an open document. A document opened from a conversation keeps it as the assistant; one opened from the Library (or a launcher note result, `open_article`) starts with the assistant closed; otherwise the owner's pane choice is kept, so re-opening the same document after Apply, Undo, refresh, stream events or a theme change changes nothing. New chat and a sidebar chat that is not about the open document use `open_primary(Chat)`.

Pane policy `arrange(width) -> Arrangement` (pure, never writes preferences): a document beside the assistant needs 560 + 1 + assistant width. If that does not fit, the sidebar draws as its rail unless deliberately expanded; if it still does not fit, the assistant becomes a temporary sheet over the document, closed by default, so the document keeps its whole region. A document alone also keeps its 560 region before the sidebar does. At 1280 the expanded sidebar, a 734 pt document and the 320 assistant fit; at 900 the document opens dominant (843 pt) with the rail and a closed sheet. `settle()` clears the sheet flag while docked so a later narrowing starts closed. Resize: drag the assistant or sidebar edge; keyboard alternative: the assistant header's width menu (Narrow 280 / Standard 320 / Wide 400).

## Shell rendering (`threads_shell.rs`)

- Sidebar (224, `sidebar` role): `brn` + collapse toggle in the traffic-light strip; New chat, Search (opens the ⌘K launcher), Library, Inbox (needs-you count, also in its accessible name), Projects + up to 5 recent projects, Recent chats (needs-you chats first with a quiet mark, then the existing bounded open list), All chats · N, workspace menu (Settings ⌘,, Keyboard shortcuts, Recent changes, Last work status, Quit BRN). Rows are 36 pt, 13 pt text, line icons, quiet `active` selected background, ellipsis with full name as accessible name and tooltip. Chat rows are keyboard rows with a visible ⋯ and the same right-click menu (`tiles::thread_menu`).
- Rail (56): the same destinations as labelled icon controls with tooltips; Inbox shows a dot.
- Header (56): object title (`tile-title-{kind}`, announced with its kind) and the view's local actions; empty parts move the window, double-click zooms. Local actions: work status/Stop/queue waiting (every view), Note: Save status, Note ⋯, Assistant toggle; Chat: the existing chat controls; Library: + menu; Projects: project ⋯; Inbox: For you / Tasks switch; Settings: Done · Esc. Unsaved document state also shows in other views' headers.
- Assistant pane: small header (Assistant · chat name, width menu, Close / Back to document in the sheet), chat controls row, the existing Chat body.
- Notice: one inline notice under the header, same precedence (`notice.rs` unchanged).
- Launcher: same items and keys, restyled as a centred search surface (≤ 640 wide, `surface_raised`, card radius, 36 pt rows).

Colors use palette roles only (`sidebar`, `surface_raised`, `active`, `hairline`, `border`, `accent`, `muted`, `ink`, …); sizes come from `theme::metrics`. No raw color literals were added.

## Keyboard

| Key | Now |
|---|---|
| ⌘K, ⌘S, ⌘↵, ⌘L, ⌘N, ⌘T, ⌘/, ⌘⇧R, ⌘., ⌘,, ⌘Q | Unchanged |
| ⌘0 | New chat (was Home) |
| ⌘\ | Collapse to rail / expand (acts on what is drawn; expanding from a responsive rail is deliberate) |
| ⌘[ / ⌘] | Focus sidebar → workspace → assistant (visible regions only; global and Input contexts) |
| Escape | Launcher, menu, dialog, stacked comments, then the compact sheet, then Settings; never stops work or discards a draft |
| ⌥1–⌥3, ⌘⇧↵ | Retired |

The Keyboard shortcuts dialog (18 rows) describes the above. Dialog focus return (`open_dialog`) is unchanged.

## Retired mechanisms and where the outcome lives

| Retired (CAP) | Outcome now |
|---|---|
| CAP-002 ⌘\ chat-list toggle | ⌘\ sidebar/rail toggle; Recent chats always in the sidebar |
| CAP-003–007 nav pills | Sidebar Recent chats, Library, Projects, Inbox (Tasks via Inbox switch), workspace menu → Recent changes |
| CAP-008 / CAP-039 Home and ⌘0 | Overview body is the Inbox (For you); ⌘0 opens New chat; first-use delegation card shows on the New chat surface |
| CAP-009–011, CAP-033 launcher button | Sidebar Search and ⌘K, same launcher items/keys |
| CAP-012 / CAP-043 Settings gear | Workspace menu → Settings and ⌘, |
| CAP-013 Settings Done · Esc | Header Done · Esc, Escape |
| CAP-014 Needs-you badge and menu | Sidebar Inbox count; needs-you chats lead Recent chats; Inbox For you rows open the chat |
| CAP-015 status-bar Save pill | Note header Save status (`status-save`); unsaved state also in other views' headers |
| CAP-016 notice banner | One inline notice under the header |
| CAP-017 Last work status / worker status | Header work status + Stop (any view); workspace menu → Last work status |
| CAP-018 Keys and mouse help | Workspace menu → Keyboard shortcuts; launcher command "Keyboard shortcuts" |
| CAP-019 Layout status menu | Removed; panels are routes, the assistant has its toggle |
| CAP-020 / CAP-044 presets Write/Chat/Solo | Document with assistant / conversation / document alone, chosen by navigation and the Assistant toggle |
| CAP-021 Open beside / close splits | Supporting pane beside the document (Chat, Linked, Compare, Connections); one at a time |
| CAP-022 / CAP-023 panel headers and ⋯ panel menus | Workspace header; object menus (Note ⋯, project ⋯) keep their sections |
| CAP-024 / CAP-042 maximize | Collapse sidebar (⌘\) + hide assistant; the document then centres its measure |
| CAP-025 close panel | Close assistant; navigation replaces the primary view |
| CAP-026 / CAP-027 swap and gap resize | Assistant/sidebar edge resize with a width menu; no swapping |
| CAP-028 / CAP-046 click focus, ⌘[ ⌘] | Click a region; ⌘[ ⌘] between sidebar, workspace and assistant |
| CAP-029 empty panel row, tile scroll | Removed; the workspace always has one primary view |
| CAP-030 compact window | Width-based pane policy (`arrange`) |
| CAP-031 note toolbar/review folding | Unchanged, driven by the arranged document width |

The launcher's `Command::Layout` items were removed. `TileKind` labels now read Recent chats, Library, Inbox, Recent changes.

## Tests removed or replaced

All counts are the `brn-desktop` binary test target with `native-ui,native-test-support`.

| Before | After | Reason |
|---|---|---|
| layout unit tests (presets, areas, swap/move, gap resize, maximize/cycle, `panel_widths`, `margin_mode` worked thresholds, Solo margin, toolbar/review folding by tile weights) | 15 shell-model tests: navigation never accumulates; document-from-chat vs Library defaults and kept choices; supporting kinds; New chat from a document; Settings return; sidebar toggle/clamp; 1280 fit; collapse-first then closed sheet; deliberate sidebar; sheet open/close keeps preference; width range survives navigation; document keeps its region; focus order; margin mode by width; narrowing from docked | Mechanism replaced; the user-visible rules are asserted on the new model |
| `layouts_resize_swap_maximize_and_tile_menu_work_by_mouse_and_keys` | `shell_sidebar_assistant_and_focus_work_by_mouse_and_keys` | Same journey on the shell: retired chrome absent, assistant toggle, edge drag and width menu, ⌘\, ⌘[ ⌘] with composer/sidebar focus, Settings via menu + Escape, ⌘0 |
| `normal_navigation_stays_bounded_and_explicit_splits_are_reused` | `sidebar_navigation_replaces_the_primary_view_and_supporting_views_are_reused` | Bounded navigation; supporting view reuse |
| `settings_and_solo_restore_arrangement_and_preserve_dirty_input` | `settings_and_the_assistant_toggle_restore_arrangement_and_preserve_dirty_input` | Dirty input and arrangement survive Settings and the toggle |
| `explicit_offscreen_split_and_keyboard_focus_are_revealed` | removed | Tile-row horizontal scrolling no longer exists; focus reveal covered by the shell journey |
| `panel_headers_name_the_object_and_hold_one_menu_with_a_panel_section` | `the_workspace_header_names_the_object_and_holds_its_local_actions` | Header naming and object menu; asserts the Panel section is gone |
| `keys_help_names_panels_menus_dialogs_and_the_settings_theme` | `keyboard_shortcuts_name_the_shell_regions_menus_dialogs_and_the_settings_theme` | New rows; asserts no tile/maximize/panel/⌥/split/solo wording |
| `panel_close_and_maximize_controls_name_their_panel` | `shell_controls_name_what_they_act_on` | Accessible names of sidebar, rail, assistant controls |
| `a_narrow_list_header_keeps_its_title_and_its_menu_opens_beside` | `a_compact_window_shows_the_assistant_as_a_sheet_and_escape_returns` | 900×600 policy: rail first, closed sheet, sheet does not squeeze the document, Escape/Back return, preference kept |
| `write_at_900x600_has_no_horizontal_overflow_and_the_launcher_fits` | `a_document_at_900x600_stays_dominant_and_the_launcher_fits` | Same 900×600 overflow and launcher bounds |
| `compact_chrome_shortens_the_needs_badge_…` | `the_inbox_count_and_search_keep_their_full_names_in_the_sidebar_and_rail` | Count/name now on the Inbox row |
| `narrow_settings_and_global_controls_remain_reachable_by_scrolling` | `narrow_settings_and_shell_controls_remain_reachable` | Same Settings reveal journey; shell controls in window |
| `narrow_workbench_scrolls_tiles_…`, `new_chat_from_solo_note_menu_…`, `reopening_current_review_from_solo_chat_…`, `a_narrow_notes_list_marks_…` | renamed equivalents | Same assertions with sheet / lone-view / Library terms |
| Review/banner stacked-comment tests at 900×600 (5) | same tests at 620×600 (one at 620×490) | At 900 the document is now dominant (Wide margin), so the stacked arrangement they guard occurs at a narrower window; assertions unchanged |
| `comment_placement_follows_the_note_width` | same test, new width table (620 Stacked, 900 Wide, 960 Narrow, 1050 Wide, 1150 Narrow, 1250 Wide) | Widths follow the pane policy; placement assertions unchanged |
| status-bar/Layout-menu/needs-badge clicks in ~20 behavior tests | sidebar, Inbox, workspace-menu and header equivalents | Same behavior through the new entry points; "Idle" is no longer permanently shown, so two tests assert its absence when nothing waits |

The pre-change suite was not run separately; the first run after the change executed 386 tests + 1 ignored (318 passed, 68 failed before migration). After rebase onto S1: 388 passed, 0 failed, 1 ignored.

## Commands and results

Rust 1.98.1, `--locked`, `--jobs 6`, `TMPDIR` under `/private/tmp`, document helpers built with `scripts/build-document-helpers.sh --offline --jobs 1` and exported as `BRN_XBERG_HELPER` / `BRN_XBERG_HELPER_BRN`. Final run at `c64e4397`:

| Command | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo clippy -p brn-writing-ui --features native-ui,native-test-support --all-targets -- -D warnings` | pass |
| `cargo test -p brn-writing-ui --features native-ui,native-test-support` | 48 + 12 passed |
| `cargo clippy -p brn-desktop --features native-ui,native-test-support --all-targets -- -D warnings` | pass |
| `cargo test -p brn-desktop --features native-ui,native-test-support` | 388 passed, 1 ignored |
| `cargo build -p brn-desktop --features native-ui` | pass |
| `git diff --check` | clean |

`scripts/verify-desktop-shell.sh` itself was not run end to end; its steps were run individually as above (without `--offline`).

## Native observation

`target/debug/brn-desktop --data-dir /private/tmp/brn-s2-data.*` (fresh, synthetic: the reading-quality fixture article, three notes, three chats, one project, created with the `brn` CLI). Window-only captures (`screencapture -l`) in `/Users/evokessler/repos/brn-ux-rebuild-artifacts/shots/s2/`:

`{light,dark}-{1280,900}-{new-chat,library,settings,document-assistant-closed,document-assistant-open,sidebar-collapsed}.png` (24) and `postrebase-1280-{settings,document-assistant-closed}.png`. Light and Dark were chosen in Settings. The 1280 window was 1280×912 (screen height limit; requested 928); the 900 window 900×628. The 24-shot matrix was taken before the rebase onto S1; the two post-rebase shots confirm the shell with S1's Appearance section.

Observed and fixed during the pass: selection tint too strong on sidebar rows (now `active`), header title colliding with traffic lights beside the rail, sheet reopening after narrowing from a docked assistant. Observed and left: see gaps.

## Known gaps

- Chat rename and delete do not exist as services; the chat menu offers the existing Open, Archive/Unarchive, Open note, Move to project.
- Pinning is not implemented (recent projects/chats only).
- Layout persistence across restarts is not wired (lead).
- The 900 sheet shows the assistant over the document; the chat body inside (skill chips, composer row) is not restyled (S3). Document canvas, Library list, Inbox and Settings bodies are unchanged (S4–S6); Settings still says "Keys and mouse".
- Opening a proposal from chat at 900 keeps the sheet closed (document dominant); the Assistant button opens it.
- "Ask about this document" affordance is the header Assistant button, not a separate quiet prompt (S4).
- Sidebar width has drag resize only (no keyboard alternative); the assistant has the width menu.
- Every chat row shows its ⋯ (keyboard reachable, not hover-only); visual weight may need S3/S5 review.
- Launcher result groups still say "Notes" (S5/S6).
- Large text/scaling, IME, VoiceOver, and a 5,000-note responsiveness check were not exercised natively.
