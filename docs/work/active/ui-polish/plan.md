# UI polish: critique and design direction

**Date:** 14 October 2026. **Owner request:** make BRN feel polished, calm and easy to understand by improving hierarchy, clutter, wording, consistency, reading/writing comfort, interaction and incomplete states, while keeping the product model, native macOS character and every safety guarantee. No publication, merge or release.
**Baseline:** designed against `4b41f190` (`origin/main`, PR #143); before implementation the branch was fast-forwarded to `771116f0` (main with #144 document conversion and #145 helper sub-agents). Since that move, the target's comment wording sits at lines 388 and 390 and the Settings theme hint sits at `threads_tiles.rs` about line 1180. Isolated branch `work/ui-polish` at `.worktrees/ui-polish`. **This phase:** critique and design, revised after design review rounds 1–4 ([review](design-review.md); responses at the end) and the owner decisions under Open questions. Round 5 left 1 MEDIUM and 4 NIT, carried into the [implementation plan](impl-plan.md). No source changed.
**Stack (locked):** Rust 1.98.1; `gpui-kit = "=0.7.3"` (Kit components 0.7.3, GPUI 0.3.9) as recorded in `Cargo.lock`. No version bumps or new dependencies. Kit 0.7.3 has no editor selection bounds, and its `Diff` component is a line-based patch viewer, so the prose review stays BRN's own.
**Authority:** [canonical target](../../../architecture/threads-target.md), [behavior contract](../../../architecture/threads-behavior.md), [invariants](../../../architecture/invariants.md) and [status](../../../status.md). Accepted UI precedent: [tile UI polish](../tile-ui-polish/evidence.md), [writing polish](../writing-polish/spec.md) ([evidence](../writing-polish/evidence.md)), [standalone chat](../chats-and-projects/evidence.md), [Projects](../projects/evidence.md), [Omakase spec](../omakase-ui/spec.md).

## Observation record

**Native-observed: how the baseline app renders.** Not observed: pointer and keyboard interaction, focus rings, hover, tooltips, open menus or IME.

- **Build.** `CARGO_TARGET_DIR=<worktree>/target cargo build -p brn-desktop --features native-ui --locked --offline` passed in 1 min 58 s; only the existing `block 0.1.6` future-incompatibility warning appeared. The binary was wrapped by `scripts/make-macos-app.sh` in uniquely identified bundles. Synthetic data lives in `/private/tmp/brn-ui-polish-design/{seeded,empty}`, outside Git. Headless startup reported 71 and 2 records.
- **Synthetic data.** [`seed-fixture.rs.txt`](observation/seed-fixture.rs.txt) ran once as a temporary `brn-threads-app` example through the shared `Workspace` service, then the file was deleted; `git status` stayed clean. It creates:
  - 18 general notes (including a long guide and a protected, confirmed decision) plus one project document
  - a writing note with three passage comments and a scripted pending whole-note candidate (not model output)
  - nine active chats (long, needs-you question, failed run with a paused queue, archived, project chat) plus one in Trash
  - 5 tasks, one in each state
  - one project with instructions, a document and a chat, plus a second project in Trash
  - a chosen backup folder

  The provider is `offline-fixture`, with no credentials, and no provider request was made.
- **Blocked routes, recorded exactly.**
  - `screencapture -x` → `could not create image from display`
  - `osascript` System Events keystroke → error 1002, `osascript is not allowed to send keystrokes`
  - `AXIsProcessTrusted()` → `false`
  - Posted CGEvents were not delivered to the app.
  - A raw, unbundled binary opened no visible window; the `.app` bundle did.
- **What worked.** ScreenCaptureKit window capture ([`wincap.swift`](observation/wincap.swift)) succeeded once the app was activated.
- **Navigation without input.** Because input could not be synthesised, navigation used an observation-only harness ([`observe-harness.rs.txt`](observation/observe-harness.rs.txt)) compiled into a scratch export outside Git. It reads commands from a file and calls the same view methods that clicks and shortcuts call: open a tile, note, chat or review, Read, theme, window size, launcher, dialogs. [`snap.sh`](observation/snap.sh) and [`input.swift`](observation/input.swift) are the remaining helpers. The harness is not part of the product and must not be committed as source.
- **Captured.** 47 window PNGs at 2×, full set in `/private/tmp/brn-ui-polish-design/shots/`. Twelve 1× copies are in [`before/`](before/). Light is Sammal; dark is Virmalised or Raba.

| Journey | Light | Dark | 900×600 |
|---|---|---|---|
| First run, empty states | 41 Write, 42 Home, 43 Notes, 44 Tasks, 45 Changes, 46 Projects | — | 47 |
| Seeded shell, Home | 01, 02 | 30 (Raba) | — |
| Notes list, Write/Read, long and protected notes | 03, 04, 05, 06, 07 | 26 | 37 |
| Candidate review (changes / result) | 08, 09 | 27 | 32 (Write + review), 34 |
| Chats: long, needs you, failed + paused queue, All, archived | 21, 22, 23, 24, 25 | 29 (Raba) | 33 |
| Send into paused queue (notice) | 39, 40 | — | — |
| Tasks, Changes | 10, 11 | 28 | 35 |
| Projects, Trash, delete preview | 12, 13, 14 | — | — |
| Settings General / Models / Delegation / Data | 15, 16, 17, 18 | — | 36 |
| ⌘K launcher, keys help | 19, 20 | 31 (Raba) | 38 |

**Not captured:**
- live streaming, Stop/Steer, or the Quiet-off trace
- attachments and Keep in BRN
- the Save as decision and Edit note dialogs
- Connections, Knowledge check and Compare
- export or backup success and failure (these need the native picker)
- open menus, tooltips, focus and drag

The desktop app has **no backup Restore UI** (restore is `brn --data-dir NEW restore --source BACKUP`), so there is nothing to capture.

**Fixture artifacts, not findings:**
- "Saved instruction is ready to continue" appears because the fixture posted owner messages without runs.
- The failed run is a synthetic Run record.
- Shot 40's message joined the paused queue, so nothing ran.

## Critique

1. **Too much chrome competes with the work.** There are four horizontal bars: title (brand, search, Write/Chat/Solo, needs-you), navigation (destinations, Changes, ⚙, Open beside ＋) and status (`○ Idle`, save pill, `⊞ custom`, `◐ Sammal`, `? keys`), with notices between them. Every panel adds a header, and the Note tile adds two toolbar rows on top. The theme picker appears in three places, layout in two, and save state in two (`threads_shell.rs`; 01, 02).
2. **Boxes inside boxes.** `panel()` draws a full border in the strong `border` color, and `heading()` puts an 18 px title on every item. The results are nested bordered cards in Tasks, Changes, Home and Settings, and full-width outline buttons used as list rows: Home recent notes, every "Undo this change", Settings actions (`threads_ui.rs`, `threads_tiles.rs`; 02, 10, 11, 16–18).
3. **Primary actions are unclear or duplicated.**
   - The Note footer shows two filled primaries ("Review rewrite" and "Rewrite with AI") across three rows (`threads_workbench.rs`; 04).
   - The review bar has seven equal controls on two rows with mixed case ("keep for later", "dismiss", "compare", "Read result") (`threads_writing.rs`; 08).
   - Each task shows four equal buttons, and Open tasks offer a no-op "Open" (`threads_tiles.rs`; 10).
   - Destructive actions look neutral: "Delete…" sits among six ghost buttons, and "Move project to Trash" is a plain outline (`threads_projects.rs`; 12, 14).
4. **Status has no tone.**
   - The single `notice: String` renders errors and info identically.
   - "Failed" uses the same small square as other run states (23).
   - The palette has no danger, warning, success or info roles, and `theme::apply` leaves Kit's tone channels at their defaults, so a Kit `Tag` or danger `Button` would not match the theme (`threads_theme.rs`).
   - A message sent into a paused queue says only "Message saved in the queue…", without saying it will not run until Resume (40).
   - Every chat shows the global queue with items from other chats (21–25).
5. **Lists lose their titles.**
   - `row()` keeps metadata at full width (`flex_shrink_0`) and has no ellipsis. Needs-you titles disappear behind their reason ("Choose the pilot start dat…" with no title), and other titles break mid-word (01, 21, 24).
   - The A–Z grid takes three rows of the Notes column, and a three-line Topics explainer sits above Recent every time (03, 37).
6. **Implementation language leaks.**
   - Changes and Home list "Chat control: recover paused queue · Chat controls · version 7" receipts, one per app launch, each with an Undo button (11, 42).
   - Timestamps are raw ISO UTC (`2026-10-14T19:47:39.046Z`).
   - Copy says "tile" ("Ask in the chat tile…"), and both "actions" and "Tasks", and both "What changed" and "Changes".
   - Panel titles are lowercase ("note · Pilot proposal", "chat · new chat").
   - The model label reads "· gpt-6.1-sol" or "chatgpt · …" (22, 41).
7. **Repetition and long explanations.**
   - Archived chats show Unarchive three times: header, body and composer (25).
   - The delegation text is a five-sentence paragraph (17, 41).
   - The review footer is a run-on technical sentence (08).
   - The Tasks and Changes tiles open with a permanent explainer line.
8. **Unfinished empty, disabled and loading states.**
   - Empty Tasks shows an empty card plus five empty group cards (44).
   - Most empty states are a single muted sentence with no action (43, 45, 46).
   - "Edit note" is greyed with no visible reason (04).
   - Loading appears only as inline text ("preparing search…").
9. **Mixed controls and icons.**
   - Unicode glyphs (⤢ ⤡ ✕ ⚙ ◐ ⊞ ● ○ ◌ ■ ＋ ▾ ⋯) mix with one Lucide icon (Quiet's eye), and ⚙ and ⤢ render tiny.
   - Selected states differ between pills, bordered Settings tabs and full-width Delegation buttons.
   - Control heights vary from 28 to about 36 px (01, 15–17).
10. **Reading and writing comfort.**
    - Write shows a code-editor line-number gutter in prose, and so does the review diff (04, 08).
    - Text lines run the full tile width: about 90 characters per line in the Chat preset, and far more when maximized (21).
    - The empty comments margin still takes about 250 px ("Select a passage to comment.") (06, 07).
    - Content gets about half of the Note tile's height at 860 pt and about 260 pt at 600 pt (04, 32).
11. **Small windows clip and wrap.**
    - At 900×600 the third panel is clipped off the right edge with no affordance ("chat · P…", "Ask or delegat"), because the Note's minimum width plus the comments margin exceeds the space (32).
    - Composer controls wrap onto two rows (35, 47).
    - The launcher is a fixed 580 px wide, runs to the window bottom even with four results, and overflows at small sizes (19, 38).

## Design direction: a quiet document workspace

The note text, the chat transcript or the list being chosen from is the brightest thing on screen. Chrome recedes to quiet bars and per-panel headers. **Each panel has at most one filled primary button, chosen by its current state; the chat composer's Send is the only addition.** Secondary actions are ghost buttons, and rare ones go in the panel's ⋯ menu. **Destructive** actions are those that lose the owner's own content with no Undo or Trash restore (Discard edit), or move content to Trash (Move project to Trash). They end in "…" and confirm in a dialog whose confirm is danger-styled. Reversible actions stay one click: Archive, Cancel task (Undo in Changes), Dismiss proposal (the note is unchanged and the rewrite can be retried). Status is a tone: icon, word and tint, never color alone. Borders separate rather than box; cards are reserved for things the owner must act on (needs you, failure, proposal ready, confirmation). Consequences stay in visible text at decision points that are irreversible or that start or replace work during recovery (see Copy guidelines → Consequences).

### Tokens

All tokens and shared builders move into a new module, `crates/brn-desktop/src/threads_components.rs` (`mod components`). The `threads_ui.rs` and `threads_shell.rs` helpers (`button`, `pill`, `panel`, `heading`, `section`, `metadata`, `empty`, `item`) become thin wrappers or are replaced, keeping the element IDs and accessible labels of every control that stays visible.

- **Spacing:** a 4-point scale of 4, 8, 12, 16, 24 and 32 px, plus 2 px hairlines.
  - 4–8 inside a control or row
  - 8 between related items
  - 16 between groups
  - 24 between page sections
  - padding 8 in navigation lists, 12 in tile content, 16–24 around reading columns
- **Type** (the accepted roles are unchanged; the system font stays, and Menlo stays for code and the wordmark):
  - 12 px: metadata, section labels and chips
  - 14 px: UI, lists and controls
  - 16 px: prose (editor, chat answers)
  - 18 px: page and dialog headings only

  Weights: 400 body, 500 selected navigation, list titles and section labels, 600 headings and panel titles. No ALL CAPS. Read keeps Georgia, the semantic Markdown heading sizes and the 0.95 rem paragraph gap.
- **Control heights.** Kit 0.7.3 sizes are XSmall 20, Small 24 and Medium/Large 32 px; there is no 28 px size, and `Size::Size(px)` sets only padding on labelled buttons.

| Token | Use | Kit realization |
|---|---|---|
| 24 px | icon buttons in headers and rows | icon-only `Button` with `.with_size(Size::Small)` |
| 28 px | toolbar, ghost and segmented controls | `.compact().h(px(28.))`, as `pill()` does today |
| 32 px | form buttons and inputs | Kit default (Medium) |
| 64 px | composer input | unchanged (accepted) |
| chip | status chips | `Tag::custom(tone_bg, tone, hairline)` at the default size (12 px `text_xs`) with `.rounded(px(4.))`; neutral chips `Tag::custom(alt, muted, hairline)`. Not XSmall/Small (10 px text) and not `.outline()` (transparent background). |

- **Bar heights:** title bar 38, navigation bar 32, panel header 36, toolbar 36, status bar 28.
- **Radii:** 4 for chips and tags; 6 for controls and inputs (`theme.radius`); 8 for panels, dialogs, popovers and the launcher (`theme.radius_lg`; panels go from 6 to 8).
- **Color roles** (`Palette` in `threads_theme.rs`; all five themes keep their base values):

| Role | Use | Rule |
|---|---|---|
| `paper` / `tile` / `alt` | window, panels, bars and hover groups | unchanged |
| `ink` / `muted` | text / metadata | `ink` ≥ 4.5:1 on tile, alt, paper and every tint (existing test). `muted` only on tile, alt, paper and hover. Text on `selection` or any `T_bg` tint is `ink`: computed with the theme's `mix`, `muted` on `selection` is 2.68 (Virmalised), 3.62 (Raba) and 4.36 (Läänemeri), and 3.5–3.9 on the dark-theme tone tints. |
| `border` | inputs and focused outlines | unchanged |
| `hairline` (new) | separators and panel outlines: `mix(tile, border, 0.45)` | replaces `border` everywhere else |
| `accent`, `primary_fg`, `hover`, `active`, `selection` | the one primary button, focused panel title and underline, selection, links, focus ring, needs-you badge | never for headings, chips or status text |
| `add`, `delete`, `comment(_strong)` | review and comment tints | unchanged |
| `danger`, `warning`, `success`, `info` (new) | icon and text color for a tone; `danger` also fills the destructive-dialog confirm | ≥ 4.5:1 on tile, alt, paper and the tone's own tint. Tone-colored text never sits directly on `selection` or `active` (measured 3.53–4.35:1 there, for example Paber warning 3.53 and Virmalised danger 3.63). Inside a selected row, status shows as a chip on its own tint, or as `ink` text with a tone icon (icons pass 3:1 everywhere). |
| `danger_bg`, `warning_bg`, `success_bg`, `info_bg` (new) | banner and chip tints | ink ≥ 4.5:1 on the tint |

  Starting hues (light / dark): danger `b63629` / `f3938b`; warning `8b5d00` / `e3b341`; success `2a7349` / `6fcf97`; info `2f6690` / `79b8ff`. Tints mix into `tile` at 0.14 (light) or 0.22 (dark). These values were checked with a script that reuses `threads_theme.rs`'s `mix` and WCAG formula: each tone is ≥ 4.6:1 on tile, alt, paper and its own tint in every theme, `ink` on each tint is ≥ 7.4:1, and `tile` on the danger fill is ≥ 5.7:1. The previous starting hues failed on light tints and alt (3.95–4.41), and dark danger failed on Raba's tint (4.44). The tests stay authoritative; adjust lightness only if they fail.

- **Kit tone channels.** For each tone T in danger, warning, success and info, `theme::apply` sets `T` and `button_T` to `pal.T`; `T_foreground` and `button_T_foreground` to `pal.tile`; `T_hover` and `button_T_hover` to `mix(pal.T, toward, 0.14)`; and `T_active` and `button_T_active` to `mix(pal.T, toward, 0.26)`, with the same `toward` the accent uses. `Theme::update` carries these into the tokens that `Button::danger()` and `Tag` read.
  - `Button::danger()` is the only filled tone button, and it appears only as the confirm of a destructive dialog whose title states the consequence. Filled warning, success and info buttons are not used.
  - Contrast tests, all five themes:
    - `tile` on `danger` and on `danger_hover` ≥ 4.5
    - each tone on `tile`, `alt`, `paper` and its own `T_bg` ≥ 4.5
    - `ink` on `selection`, `hover` and each `T_bg` ≥ 4.5
    - `muted` on `tile`, `alt`, `paper` and `hover` ≥ 4.5 (lowest today: Virmalised hover, 4.67)
  - Raba keeps its red accent. Errors there stay distinguishable through the tone icon, the title and the tint, and red fill appears only inside a dialog titled with the consequence.

### Shared components

1. **`icon_button(id, icon, label)`.** A 24 px (`Size::Small`) ghost Kit `Button` with a Lucide icon, `accessibility_label(label)` and a tooltip that includes the shortcut (`tooltip_with_action` where an action exists). It replaces the glyphs listed in critique 9.
   - Kit's default set covers Maximize, Minimize, Close, Ellipsis, Settings, Plus, Search, Eye/EyeOff, Square (Stop), LoaderCircle, CircleAlert, TriangleAlert, Info, CircleCheck, Undo2, Copy, FileText, Folder, Inbox, BookOpen, PanelRightOpen and ChevronDown.
   - Extras are selected with `gpui_kit::assets::icon_assets!(BrnIcons, [MessageSquare, Lock, House, ListTodo, Paperclip, Archive, Keyboard, TableOfContents, Type])`, composed with `Assets` in one `AssetSource` in `threads_ui::launch`. This is supported Kit composition, not a new dependency. (`table-of-contents.svg` and `type.svg` are in `gpui-kit-assets` 0.7.3 `assets/icons`.)
2. **`panel_header`** (today's `tile_header` in `threads_shell.rs`). It shows a kind icon and the object title in sentence case with an ellipsis ("Pilot proposal", "New chat", "Chats"; no `note ·` prefix; the accessible label stays "Note: Pilot proposal"), up to two contextual controls, then ⋯, Maximize and Close.
   - **One ⋯ per panel, in its header.** Its first section holds the panel's object actions for the current state: the Note panel's existing `note-menu` items; the review actions during review; the selected project's actions in Projects. A "Panel" section follows with the right-click menu's items (Maximize, Move left, Move right, Open beside, Close other splits, Close), so splitting is discoverable without right-clicking. The Note panel's ⋯ keeps the `note-menu` ID; other panels use `tile-menu-{kind}`. The right-click menu is unchanged. List rows may have their own row ⋯ (Tasks); toolbars and bars have none.
   - During review the Note header keeps the note title (the review bar states the review). A chat started from a note reads "New chat about {note}".
   - Focus keeps the accepted single accent: accent title plus a 2 px accent underline.
   - A panel in `Area::Beside` keeps today's "split" cue as a neutral "Split" chip after the title, with the tooltip "Opened with Open beside. Navigation doesn't replace it." It is the only sign that a panel is outside the reuse model.
3. **`toolbar`.** One row under a header, with a hairline below. On the left a `segmented` mode switch and the mode's controls; on the right a status chip and secondary controls. At most one toolbar per panel. The Note toolbar's collapse order is in the small-window rules.
   - **`segmented(id, buttons)`** is a Kit `ButtonGroup` of 28 px compact ghost `Button`s with no group-level `on_click`, so each segment keeps its own ID, handler and tooltip, and Kit reports the selected segment as pressed. Kit `TabBar` and `RadioGroup` are not used for these: in 0.7.3 both replace child IDs with indexes, which would break `layout-*`, `note-write`/`note-read`, `settings-section-*` and `delegation-*`.
   - When a single toggle becomes a pair, the old ID stays on the segment it switched to, and the other segment gets a new ID. Example: `review-read-result` is the Result segment, and Changes is `review-show-changes`.
4. **`overflow_menu`.** An Ellipsis `icon_button` with a Kit `DropdownMenu`, also used for ▾ menus.
   - Items are sentence case. A disabled item appends its reason, as "Comment on selection · save first" does today.
   - Destructive items come last after a separator, end in "…" and open a confirming dialog.
   - Tests address items by accessible label (the existing `choose` helper). A control that moves into a menu retires its element ID; the full list is under Focus, keyboard and scrolling rules.
5. **`section_header(label, count, trailing)`.** 12 px, weight 500, muted, 16 px above; optional trailing ghost such as "All" or "Show 3".
6. **`list_row` / `list_row_two_line`.** 28 px (one line) or 44 px (title plus reason).
   - The title truncates with an ellipsis and never yields to metadata.
   - Metadata is capped at 40 % of the width and truncates.
   - Selection uses the selection tint plus weight 500, and the selected row's metadata switches from `muted` to `ink` (see color roles). Hover uses the hover tint and keeps `muted` metadata.
   - Each row sets `aria_selected(selected)` (GPUI 0.3.9 `div.rs`), so selection reaches assistive technology as well as color and weight.
   - IDs and context menus are unchanged.
   - Needs-you rows use the two-line form.
7. **`empty_state(id, icon, title, body, action)`.** A muted 20 px icon, a 14 px/600 title, at most one 12 px sentence of body, and at most one button (primary only when it is the obvious next step). Other routes are named in the body text. It keeps the existing IDs (`threads-empty`, `notes-empty`, `actions-empty`, `history-empty`, `home-*-empty`, `tiles-empty`).
8. **`banner(id, tone, text, action)`.** BRN's own row: the tone tint (`*_bg`), the tone icon (Info, CircleCheck, TriangleAlert or CircleAlert) in the tone color, the text in `ink`, optional Kit `Button` actions and an optional dismiss `icon_button`. Kit `Alert` is not used because in 0.7.3 it takes only a message, so it cannot hold an action, a BRN dismiss ID or the restore tooltip. Banners never auto-dismiss.
   - **Workspace notice:** full width; keeps the `workspace-notice` ID and the `dismiss-notice` control with its "restore from Layout → Last work status" tooltip.
   - **Note tile banners** sit between the toolbar and the text. They never move into ⋯ and are outside the chrome budget.

| Banner | Tone | Text | Actions (IDs kept) |
|---|---|---|---|
| Newer saved version | warning | A newer version of this note is saved. Your visible text is kept. | Open current version (`show-current-note`) |
| Recoverable edit | warning | An unsaved edit was kept. This note is read-only until you recover it. | Recover edit (`recover-note-edit`) |
| Import summary (`note-import-summary`) | info only when the report records full coverage; warning for Partial and for "Coverage not recorded" | the existing coverage summary | Open original (`open-import-original`), Coverage details (N) (`toggle-coverage`; also in ⋯ as today) |
| Changed since your edit began (`note-stale-banner`, new ID; shown when an open edit session's base version is behind the saved note, as today) | warning | This note changed since your edit began. Your text is kept. Compare it with the saved version below, or save your text as a new note. | Save as new note (`save-buffer-copy`, moved here from the writing bar in this state), Open current version (`show-current-note`) |

   - **Stale-edit preview.** Below the stale-edit banner, the current saved note (`current_view`, as today) renders in a 180 px capped scroller (`note-stale-preview`, new ID) on `alt` with a hairline, under the 12 px label "Saved version". It replaces today's bordered `panel()`. The comparison is this inline preview: the Compare panel needs a review proposal (`compare_tile` shows "No whole-note proposal to compare." otherwise), so no Compare button is offered here.
   - **Overlap.** With an open edit, the two version conditions usually hold together. When both hold, only the stale-edit banner renders: it states the same fact plus the comparison, and it carries `show-current-note`, so that ID stays visible and unique. When only the newer-version condition holds, the newer-version banner renders as before.
   - **Banner order**, top to bottom: recoverable edit, stale edit (or newer version), import summary. Expanded coverage details (`note-coverage-details`, 120 px) stay directly under the import banner.

   - **Inline cards** (radius 6, same row): recovery card (the strongest tone among its lines; step 4), needs-you question (warning), paused queue (warning), stale or superseded proposal (warning), archived chat (neutral, in the composer).
   - **Tone state.** `ThreadsUi` gains `notice_tone: Tone` (`Info`, `Success`, `Warning`, `Error`) and `notice_tone_for: String`. The helpers `inform`, `confirm`, `warn` and `fail` set the text, the tone and `notice_tone_for` together. The banner uses `notice_tone` only while `notice == notice_tone_for` and renders Info otherwise, so a direct `notice =` write can never inherit an older tone. `notice: String` stays, so existing assertions keep working.
   - **Tone mapping.** Every site that writes an error into `notice` uses `fail(..)`, whatever its shape:
     - `Err(..) => self.notice = ...` match arms (49 today)
     - `if let Err(e) = … { self.notice = … }` blocks (13 today, among them `cancel`, `resume_chat_queue` and the project and Trash actions) and `map_err` notice writes
     - the worker-disconnect notices ("Work stopped unexpectedly…")
     - `WorkerEvent::Finished(Err(..))` ("Work stopped: …") and `WorkerEvent::Converted` with an `Err` result

     `WorkerEvent::Finished(Ok(..))` takes its tone from the recorded state of the worker's own `run_id`, read from the store as the `run_activity_status` update does (never the chat's `latest_run`, which can be an older run):
     - Completed → success
     - Failed → error
     - Interrupted, Cancelled, or still Working (status unconfirmed) → warning

     A worker without a run (export, backup, import, connect) → success. The `outcome_message` and `save_message` sites map:
     - `ApplyOutcome`: Applied → success, or warning when `receipt.needs_refresh` is non-empty; Deferred and Stale → warning; NeedsReview and Superseded → info
     - `SaveOutcome`: Saved → success; Stale, GenerationMismatch, Closed and Deferred → warning

     Refusals that arrive as plain messages, such as "Wait for current work to stop before Resume.", use `warn`. Every other notice uses `inform`. A missed site stays Info, never an older tone.
   - **Restoring status.** `last_work_notice` becomes `Option<(Tone, String)>`. The `Finished` handler stores the tone it mapped together with the "Work in X · …" text. Comparisons use the text, and Layout → Last work status restores both.
9. **`status_chip(id, tone, label)`.** A `div().id(id).aria_label(label)` wrapping the Tag described under Tokens. Chips:
   - Note status (`note-status-chip`), in every state including dirty: "Protected · v3" (info + Lock) or "Working · v3" (neutral + FileText). Below 480 pt of Note width it shows the icon and "v3", keeping the full text as accessible label and tooltip. The toolbar has no unsaved chip; the `note-unsaved-chip` ID moves to the writing bar's "Unsaved edit · recoverable" label (warning, step 3).
   - Reviewing (info)
   - Failed (danger)
   - Paused (warning)
   - Archived (neutral)
   - Needs you (warning)
   - Needs refresh (warning)

   The top-bar needs-you badge stays a primary button, as accepted.
10. **Buttons.**
    - At most one filled `primary()` per panel, plus the composer's Send.
    - List and history rows never use a filled primary. A row's main action is a compact `outline` button, and the rest go in the row's ⋯.
    - In a transcript or a proposals list, only the newest item awaiting the owner (Apply/Approve, Resume queue/Continue, Review rewrite) is primary; older ones are outline.
    - Other secondary controls are `ghost()`; `outline` is also used for form buttons in Settings and dialogs; `danger()` appears only on the confirm of a destructive dialog.
    - A disabled control states its reason in the tooltip and appends it to its accessibility label ("Rewrite with AI, unavailable: BRN is working"). Where the reason decides the next step (writing bar, review bar), it is also shown inline.
11. **Focus ring.** Keep Kit's ring (`colors.ring = accent`) on inputs and buttons, and never hide it. The focused panel is shown by its header.

### Layout rules

- **Panels.** Panels sit on `paper` with 6 px gaps, radius 8 and a hairline outline. Pages (Home, Tasks, Changes, Settings, Projects) use a content column of at most 720 px; sections are 24 px apart and hold rows, not a card per item.
- **Reading measure.** The Note Read and Write column and the chat transcript are at most 680 px wide, with 24 px gutters, left-aligned in split views and centered when maximized or in Solo. Long words and URLs wrap.
- **Panel widths.** A pure function `threads_layout::panel_widths(available, tiles, margin)` resolves the layout's weights against the same minimum widths the renderer uses, with 6 px gaps and 6 px outer padding. It only chooses content modes: the margin mode, the Note toolbar collapse and the composer label. Rendering stays flex, so resizing and dragging are unchanged. Maximized and Solo give the one panel the full width.
- **Minimum widths, per `TileKind::min_width`:**
  - Chats (`Threads`), All chats (`AllThreads`) and Notes: 170. Notes rises from 260: its rows truncate, its search field shrinks, and its A–Z strip scrolls horizontally (step 5).
  - Note: 360 plus the margin mode's width (none in Stacked)
  - Chat: 300
  - Compare: 320 (unchanged)
  - everything else: 260

  Today Threads is 170, Note and Compare are 320, and the rest are 260 (Notes and Chat included), with a 250 px margin added. The accepted Write weights (navigation 0.62, note 2.7, chat 1.45) and Chat weights stay. Every panel that can occupy the navigation area is 170, so the worked numbers below hold whichever list is open there.
- **Comments: beside the text when the Note tile has room, otherwise below it in the tile** (open question 2). They are never in Chat.
  - **Mode choice.** `threads_layout::margin_mode` picks the first mode whose minimum widths fit: Wide (a 280 px margin beside the text), Narrow (220 px beside), otherwise Stacked (no margin width).
  - **Worked numbers for Write** (navigation 170, note, chat 300, plus 24 for padding and gaps): Wide needs 1134 pt, Narrow 1074 pt and Stacked 854 pt. So 1250×860 is Wide, 1100 pt is Narrow, and 1050×700 and 900×600 are Stacked, with Chats, All chats or Notes in the navigation area.
  - **Wide and Narrow.** The margin (`writing-feedback`) is hidden when the note has no comments and no draft. The toolbar toggle still shows the count and opens it, and ⌘/ opens it with a draft. Cards keep the left comment rule and drop borders.
  - **Stacked.** A docked section (`comments-section`, new ID) sits in the Note tile between the text and the writing bar (in review, between the review text and the footer), with a hairline above. It never covers the text.
    - A 12 px label "Comments · N", then the same `writing-feedback` list in a scroller capped at 40 % of the Note body height (about 138 of 346 px at 900×600, leaving the text about 208 px).
    - It reuses the beside modes' card, draft and reattach renderers and element IDs, so every capability runs through the same code: multiple passage comments, Save and comment while dirty, saved-selection continuation, reattaching unlocated comments and the full-context rewrite. Overall feedback stays in the writing bar.
    - `toggle-margin`, with its count, is the only expand/collapse control. The section starts collapsed (session-only state) unless a draft is open; ⌘/ and `selection-comment` expand it with the new draft focused, and Escape with focus inside collapses it. Collapsing keeps drafts and scroll position, and highlights stay in the text.
    - Show passage reveals and selects the passage in the text above and leaves the section open.
    - **Height share (round-5 N3).** Below the text come `comments-section`, `writing-run-status`, `writing-proposals` or `writing-result`, then the writing bar. Outside two exceptions (a Note banner, an open stacked comment draft, both below), the text keeps at least 120 px: the reply gives way first, down to its label, then the open section, then the text. **Fix pass, owner-accepted 16 Oct 2026:** with a Note banner shown, the text still asks for 120 px and gives way last, and its floor is 48 px. That floor is reached only when the rows that never hide leave less than 120 px: the lead banner with its actions (INT-4), the saved version's first line, a failed run's full reason (U6), the reply's label and the whole writing bar. In the busiest stale state at 900×600 (stale edit, failed run with its partial reply, dirty bar, open comments) they take 332.5 of the Note's 416 px in the test text system (64 + 28 + 76.5 + 32 + 132; the bar wraps to three rows). The open section then gives way to nothing and the text gets the remaining 83.5 px. With a banner and enough room (stale edit, open comments and a dirty bar), the text keeps 120 px. Measured in [evidence](evidence.md#fix-pass-review-round-2). **Lead decision, 16 Oct 2026 (R1), extending the owner-accepted banner rule; the owner can overrule:** while a stacked comment draft is open in a short Note, the text may give way down to the same 48 px floor, and the writing bar and the whole draft input stay in the Note. The text still asks for 120 px and gives way last. At 900×600 with a draft, Retry saved rewrite and no banner, the rows that never hide (draft section 139, writing bar 103 and the run's status row) leave the text 48 px ([integration evidence](../integration-ui-reliability/evidence.md)). Every other state keeps 120 px.
- **Note tile chrome budget.**
  - Header 36 plus one toolbar 36 above the text. The toolbar is the only fixed row above the text in both modes.
  - **Sections.** Today's Write section row (up to 24 wrapping `section-{i}` pills, visible in shot 04) folds into the Outline menu (`note-outline`) that Read already has. Write and Read then share one Outline control, with the same items and the same `reveal` handler. The `section-*` IDs retire, and the `threads_focus_tests.rs` assertion on `section-0` opens `note-outline` and chooses a heading by label.
  - **Selection strip.** `selection-toolbar` stays a contextual row directly above the text, shown only while a passage is selected (Omakase-accepted; there are no selection bounds for a floating toolbar). It keeps `selection-comment` (Comment ⌘/, or Save and comment while dirty), `selection-ask` (Ask in chat) and `selection-comment-reason` while dirty. It is restyled with the toolbar tokens: 28 px controls, `alt` background, hairline below, no box border. Like banners, it sits outside the budget.
  - Below the text, one writing bar: an overall-feedback field with `auto_grow(1, 4)`, which follows its content (one line when empty, up to four as text is added), the scope summary and the actions. In the single-line state the bar is at most 72 px; growth past that is allowed.
  - In review: the header, one 36 px review bar that replaces the toolbar (narrow-width collapse in step 3), and the footer (one or two lines at 900×600; it is never clamped).
  - Note banners, the stale-edit preview, the selection strip, the expanded comments section and expanded saved inputs are outside the budget and are never hidden to meet it. Order above the text: toolbar, banners, selection strip.
- **Long content.**
  - Every single-line label truncates with an ellipsis and keeps its full text in the tooltip and accessible name. The review footer is the exception: it is never clamped or truncated. Two lines is its fit target at 900×600; it wraps further when needed, with the binding sentence first.
  - Lists keep their existing caps and counts ("Showing 100 of 412 · keep typing").
  - Transcripts keep MessageScroller and Jump to latest.
  - Each panel has one main vertical scroller. Allowed nested or capped scrollers:
    - the chat activity trace
    - the comments margin (Wide, Narrow) or `comments-section` (Stacked, at most 40 % of the Note body)
    - `review-details` (220 px)
    - `writing-proposals` (110 px)
    - `writing-result` (120 px)
    - `note-coverage-details` (120 px)
    - `note-stale-preview` (180 px)
    - `project-delete-preview` and `project-link-list`
    - launcher results and dialog bodies

### Copy guidelines

- **Case.** Sentence case everywhere (the accepted labels "Open beside", "Save as decision" and "Keep in BRN" already follow it), always starting with a capital: "Keep for later", "Dismiss", "You", "Keys".
- **Vocabulary.** Chat, not thread; Task, not action; Changes for the history view; plus Note, Protected, Archive, Project and Trash. Never tile, candidate, receipt, fence, record or operation.
- **Versions.** Versions appear as the `v3` chip. In text they appear only where authority, a conflict or a loss depends on them: the Edit note dialog ("may edit this saved note (v3)"), the newer-version and stale-edit banners, stale messages and the Discard dialog ("Your changes since v3 will be lost.").
- **Buttons.** A verb of one to three words, with "…" when a dialog or picker follows.
- **Consequences.** At decision points that are irreversible or that start or replace work during recovery, the consequence is a visible line beside the control, and tooltips may only repeat or extend it. These decision points are dialogs, recovery cards (Continue, Resume queue), banners and the review bar (the exact-text footer). Reversible one-click controls that stop work (Archive, Steer, Stop) state the consequence in their tooltip and accessible label, and the notice they produce states it again:
  - Archive (`resolve-thread`): tooltip "Archive this chat. Stops work and pauses the queue." Notice: "Chat archived. Work stopped and the queue is paused. Tasks keep their progress."
  - Steer (`chat-steer`): tooltip "Stop the current reply and start fresh with this message." Notice: "Message saved. Stopping the current reply, then starting yours."
  - Stop (`stop-work`, `stop-writing`): tooltip "Stop the current work and pause the queue · ⌘." (`stop_chat_run` pauses the queue). Afterwards the chat's recovery card shows Stopped and states what Continue or Resume queue does.

  Other explanations may live in the tooltip.
- **Explanations.** At most one sentence, shown only where the choice is made. Errors state what happened, what is safe and what to do, and "what to do" names the button the owner can see.
- **Times.** "Today 22:47" or "14 Oct, 22:47", in the Mac's time zone (fix pass: UTC read wrong on a UTC+3 machine). The full ISO value, in UTC, goes in the tooltip. The All chats Today / This week grouping is unchanged.

| Now | Becomes |
|---|---|
| Message saved in the queue. Each item runs as a separate turn with current context. | Per queue state (step 4, queue notices). |
| ■ Failed + run progress + Continue starts fresh… + "Continue with current context" / "Resume queue with current context" | The recovery card template (step 4): recorded state and reason, then what the button does. Buttons: Resume queue / Continue. |
| Instruction saved. Waiting for the current run to stop. (after Steer) | Message saved. Stopping the current reply, then starting yours. |
| reviewing the candidate · approval binds this exact text · 3 captured comments stay pending until you mark them | Per proposal shape (step 3, review footer), for example "Applying saves exactly this text. 3 comments stay open until you mark them addressed." |
| Pick a note from the notes tile or ⌘K, or create one with ⌘N. | **No note open** · Open one from Notes or ⌘K. [New note] |
| Saved. {reason} N directly derived records may need refresh. Open What changed for the affected records and a follow-up. | Saved. N related items may need refresh. Open Changes to review them. (warning tone). Fix pass (U8): the recorded reason stays in Changes and never appears in a notice; Undo reads "Undone.", and a change that stopped running work adds "Running work was stopped." |
| Your edit is recoverable. Save and comment commits it before anchoring feedback to this passage. | Save and comment saves your edit first, then comments on this passage. |
| The current note changed since this edit began. Your text is preserved; compare, then save your text as a new note if needed. | This note changed since your edit began. Your text is kept. Compare it with the saved version below, or save your text as a new note. |
| Your chats appear here. ⌘T starts one; chats stay in history until you archive them. | **No chats yet** · Start one with ⌘T. |
| Ask in the chat tile, open notes from the notes tile, or press ⌘K. | (removed; first-run empty state only) |
| Delegation paragraph (five sentences) | **How should BRN update your notes?** · Maintain ordinary notes: BRN updates ordinary notes and source references. · Review changes first: BRN proposes; you approve. · Protected notes change only with your instruction or review. (option labels unchanged) |
| Delete project — review removal · Move this project and its exclusive contents to Trash… / Move to Trash / Keep active / 5 dependent records… / Move project to Trash | **Move "Website refresh" to Trash?** · Moves to Trash: … · Stays active: … · Also moves N other items (messages, links). · Open tasks: each keeps its [Keep outside project] / [Cancel task] choice. [Cancel] [Move to Trash] |
| Tasks keep their own progress, including after a chat is archived. / Recorded changes and the same checked Undo… | (removed from page tops; used as empty-state body) |
| Undo this change · Chat controls · version 7 · 2026-10-14T20:05:38.374Z | Reason · affected titles · Today 20:05 · [Undo] |
| Directly derived work may need refreshing… + May need refresh: X + Open refresh follow-up | Warning sub-row: **May need refresh** · [X] · [Open refresh follow-up] |
| `chat · new chat`, `note · Pilot proposal`, `linked work` | New chat · Pilot proposal · Linked work |
| `· gpt-6.1-sol` / `chatgpt · gpt-6.1-sol` | gpt-6.1-sol / ChatGPT · gpt-6.1-sol |
| `Pilot proposal v1 saved` / `● Pilot proposal unsaved · ⌘S` (status bar) | Saved (muted; tooltip "Pilot proposal v1 saved · Open Changes") / ● Unsaved · ⌘S |
| Archived chat · saved history remains available + Unarchive ×3 | One composer banner: Archived. Unarchive to reply. [Unarchive] |

The delete row's "N other items" counts only removed records not already listed (not projects, chats, notes or tasks), names only the kinds present, and is omitted when N = 0.

### Small-window rules

The minimum window stays 850×600. Breakpoints come from `window.viewport_size()` during render: **Compact** below 1000 pt wide, **Short** below 700 pt tall. Compact changes title and navigation chrome only. Content modes follow `panel_widths` (Layout rules), because a fixed breakpoint leaves 1000–1140 pt overflowing.

- **Compact chrome:**
  - Search collapses to an icon button (tooltip "Search ⌘K").
  - The needs-you badge shows "● 1", keeping its full accessible name.
  - Open beside becomes icon-only.
- **Note toolbar, by Note panel width.** The thresholds below are starting values; step 3 sets them from measured label widths so the toolbar stays one row at 900×600, 1050×700 (Note about 556 pt) and 1250×860 (about 687 pt). Since the fix pass (INT-3) the renderer measures the open note's row (its labels, status chip and comment count), so a plain note keeps Edit note and Connections at widths where the widest row folds them.
  - Always visible: Write | Read, Outline (`note-outline`), the status chip and the comment toggle.
  - Below about 760 pt: the five formatting buttons fold into a "Format ▾" menu (`format-menu`) with Heading, Bold, Italic, List and Quote.
  - Below about 620 pt: Edit note and Connections move into the Note ⋯ as "Edit note…" and "Connections".
  - Below 480 pt: Format and Outline become icon-only triggers (Type and TableOfContents icons), keeping "Format" and "Outline" as tooltip and accessible label. The status chip shows its icon and vN.
  - At 900×600 in Write, the Note panel is about 406 pt (876 pt for panels, minus navigation 170 and chat 300; about 382 pt usable). The toolbar is then one row of about 290 pt, dirty or not: Write | Read about 110, two 28 px icon triggers, the status chip about 44, the comment toggle about 44, and the gaps.
- **Review bar, by Note panel width** (it replaces the toolbar during review). As for the toolbar, 620 and 480 pt were starting values. Step 3 set the thresholds from measured widths on a protected review with the full labels and a two-digit count (`ReviewFit`): the full bar needs about 915 pt and the compact one about 658 pt, so neither fits at 620 or 480 pt. The collapse order is unchanged:
  - Below about 920 pt: Keep for later and Dismiss move into the Note ⋯ review section, and the +a −r counts move into the Proposal label's tooltip and accessible name. 1250×860 (Note about 687 pt) is here.
  - Below about 670 pt: the Proposal label reads "Proposal", with the shape and counts in its tooltip and accessible name. The primary reads "Apply" or "Approve", with its full label ("Approve whole note") as tooltip and accessible label, and the Protected chip shows only its Lock icon (accessible label "Protected"). A Prepare selection that waits (every group included, or while editing) moves beside the group toggles in the details. 900×600, 1050×700 and 1100×760 are here, and the bar fits one row at 900×600 (about 378 of 388 usable pt).
  - Editing the final text and a partial group selection add controls, so the bar wraps in those two states instead of clipping one.
  - The exact-text footer is never clamped or truncated, at any width or in Short windows (Long content). It never moves: the details give way first (down to 44 px), then the review text (from 120 down to 48 px).
- **Composer, by chat panel width:**
  - Below 420 pt, the model control (`model-pill`) reads "gpt-6.1-sol · medium ▾" and truncates the model name before the effort. Its menu adds a "Reasoning effort" section with the same choices and handler as `effort_menu`. Its accessible label is "Model gpt-6.1-sol, effort medium", and `effort-pill` is not rendered.
  - At 300 pt or wider and idle, the controls stay on one row.
  - At every width, while work runs (Steer shown), Send (`ask-delegate`) reads "Queue", because ↵ then queues the message. Its tooltip is "Add this message to the queue ↵", and the notice after it follows the queue-notice table. While editing a queued item it keeps today's label. The row wraps only if it still does not fit.
- **Overflow:** when the panels still overflow, a Kit `Scrollbar::horizontal(&tile_scroll)` on the existing handle makes it visible; the focused-panel reveal is kept. Write in Stacked needs 854 pt, so the scrollbar is expected only from the 850 pt minimum to 853 pt; 900×600 is the no-overflow bound that the tests check.
- **Short:** the chat queue collapses to "Paused · 2 waiting · Resume queue", with a disclosure for the items. The state leads and never truncates; in a narrow chat the count gives way first (fix pass), with the full line as tooltip and accessible name.
- **Overlays:**
  - The launcher is `min(580, window − 64)` wide, its height fits the results (at most 60 % of the window), and results are grouped under "Chats", "Notes" and "Commands" with kind icons instead of `chat ·` / `note ·` prefixes. Archived chats keep a neutral Archived chip, and their accessible name includes "archived".
  - Dialogs are at most window − 96 tall; the body scrolls and the buttons stay visible. Today's dialogs put their buttons in the body, so every dialog's action row moves into Kit `Dialog::footer(..)` (`gpui-component` 0.7.3 `dialog.rs`), which Kit keeps unclipped while the body scrolls (its footer test). The buttons keep their IDs.

### Focus, keyboard and scrolling rules

- **Focus lands in visible content.** `focus_tile_contents` covers every destination:

| Destination | Focus target |
|---|---|
| Note | the visible editor (today) |
| Chat | the composer (today) |
| Notes | the `search` field |
| Chats (Recent, `TileKind::Threads`) | the panel body's own focus handle, then `window.focus_next()`, which lands on the Recent / All / Archived switch (`chats-recent`). Recent renders no filter field. |
| All chats (`TileKind::AllThreads`) | the `threads_query` filter, which only this view renders |
| Tasks, Changes, Home, Settings, Projects | the panel body's own focus handle, then `window.focus_next()` to the first control in visual order (Settings: the section switch). With no control, focus stays on the panel body. |

  Kit 0.7.3 keeps each Button's focus handle private (keyed by element ID), so BRN cannot focus a named button directly. The test asserts that after opening each destination, the focused element lies inside that panel. For Chats it also asserts that `threads_query` is not focused, and for All chats that it is.
- **Dialogs and ↵.** Kit 0.7.3 binds `enter` to `Confirm` in the dialog key context (`gpui-base` `dialog.rs`), and GPUI matches bindings before element key listeners. ↵ therefore reaches the dialog's `on_ok` from anywhere inside it, including from a focused Button and from a single-line `Input` (whose Enter handler propagates). Without `on_ok`, Kit's default closes the dialog. Every BRN dialog sets `on_ok` explicitly, by class:

| Class | Dialogs | `on_ok` (↵) | Submit routes |
|---|---|---|---|
| Single field | New note, project name | submits through the same function as the visible button; returns `false` on a validation error, so the dialog stays open with its error | ↵ anywhere in the dialog (the macOS default-button behavior), click, Space on the focused button |
| Multi-line form | Edit note, Save as decision, Project instructions & settings | returns `false`: ↵ in an editor inserts a newline, and ↵ elsewhere does nothing | ⌘↵, click, Space on the focused submit button |
| Pick list | Move chat, Link document | returns `false` | click a row, or Tab to it and press Space: the rows become full-width ghost Kit Buttons that keep their IDs (`project-link-doc-*`, `move-chat-*`), so they are tab stops |
| Destructive | Move project to Trash, Discard edit | returns `false` | click, or Space after tabbing to the `danger()` confirm |
| Information | Keys and mouse | Kit default (closes) | — |

  - **⌘↵ in multi-line dialogs.** Kit binds ⌘↵ to `Enter { secondary: true }` in the `Input` context, and a multi-line editor handles it by inserting a newline without propagating, so the global `cmd-enter → Ask` never fires there. Each multi-line dialog's content root therefore uses `capture_action::<gpui_kit::component::input::Enter>`: when `secondary && !shift` it calls the dialog's submit function and `cx.stop_propagation()`, so the capture runs before the editor and no newline is added. This covers every field in the dialog, including the single-line decision title. The root also keeps `on_action::<Ask>` for ⌘↵ while focus is on a dialog button (dialogs render under `Root`, outside the `ThreadsUi` element that handles `Ask`).
  - A dialog with a text field focuses it on open. Escape cancels every dialog through Kit's `Cancel`.
- **Destructive dialogs.** The title states the consequence, the body says whether it can be undone, and the buttons are [Cancel] and a `danger()` confirm in the footer. They open with Kit's default dialog focus. ↵ never confirms, because `on_ok` returns `false`, and the dialog stays open. The confirm activates by click, or by Space after the owner tabs to it. ↵ on the focused confirm is inert, and the Keys help's Dialogs row names Space as the way to press a focused button.
- **Escape.** Escape closes, in order: the launcher, an open menu, the open dialog, the stacked comments section (when focus is inside it), then Settings. Settings' handler moves from the workspace root's `capture_key_down` to a bubble-phase `on_key_down`, so an open menu or popover handles Escape first. Escape never stops or pauses work. ⌘. and the visible Stop controls stay the only stop routes, as today.
- **Menus and reachability.**
  - Every ⋯ and ▾ trigger is a Kit Button and so a tab stop. Tab to it and ↵ opens it (`gpui-base` `popover.rs` toggles on `Confirm` or mouse-down; Space reaches only the Button's `on_click`, which dropdown triggers lack). Then ↑/↓ move, ↵ activates and Escape closes (Kit `PopupMenu` bindings).
  - ⌘K reaches every destination once `Command::OpenTile(TileKind::Projects)` (label "Projects") is added to `Command::all()`.
  - Step 1 adds a keyboard test that opens a ⋯ menu and activates an item. If it shows that a focused trigger does not open its menu in 0.7.3, each menu-only action below gets a ⌘K command instead.

  Actions that become menu-only:

| Action | Old control (ID, retired) | New place | Other route |
|---|---|---|---|
| Jump to a section (Write) | `section-{i}` | Outline (`note-outline`), shared with Read | — |
| Compare side by side | `review-compare` | Note ⋯ during review | — |
| Edit final text | `review-edit` | Note ⋯ during review | — |
| Inputs (checked item) | `review-inputs` | Note ⋯ during review | — |
| Keep for later, Dismiss (Note panel below about 920 pt) | `close-review`, `dismiss-proposal` | Note ⋯ during review | buttons at wider widths |
| Task actions other than the row's main one (by state, step 5) | `wait-action-*`, `cancel-action-*`, and the non-main `done-action-*` / `open-action-*` | task row ⋯ | — |
| Instructions & settings, New document, Import file…, Link document…, Move to Trash… | `project-settings`, `project-note`, `project-import`, `project-link`, `project-delete` | Projects ⋯ | — |
| Formatting (Note panel below about 760 pt) | `format-*` | Format ▾ | buttons at wider widths |
| Edit note, Connections (Note panel below about 620 pt) | `edit-note-context`, `note-connections` | Note ⋯ | buttons at wider widths; ⌘K Connections |
| Effort (chat panel below 420 pt) | `effort-pill` | model menu | pill at wider widths |
| Move left/right, Open beside, Close other splits | right-click menu | also header ⋯ | Open beside in the navigation bar |
| Maximize (panel below 220 pt, fix pass) | `tile-max-*` (kept at 220 pt and wider, and as Restore while the panel is maximized) | header ⋯ Panel section | double-click the header, ⌘⇧↵ |

  Reload, Save text as a new note, Protect / Make working, Export and New chat for this note are already menu-only in the Note ⋯ today. **Removed:** the `status-theme` pill; theme switching lives in Settings → General, and ⌘K keeps its theme commands (open question 4).
- **Shortcut hints.** Shortcuts appear in tooltips, launcher rows and empty states, using Kit `Kbd`.
- **Scrolling and labels.** Search fields, filters, tabs and toolbars stay outside scrolling content (accepted). New icon-only controls need an accessible label. Selection is never shown by color alone.
- **Known limit.** Panel list rows are not individually keyboard-focusable (dialog pick lists are the exception above); arrow-key list navigation is out of scope.

## Prioritized improvements

Each step keeps every checked service call unchanged and ends with strict build, lint and the native test suite.

1. **Foundation** (`threads_components.rs` new; `threads_theme.rs`, `threads_ui.rs`, `threads_shell.rs`, `threads_layout.rs`).
   - Tokens, semantic roles and the Kit tone channels, with the five-theme contrast tests.
   - Icon asset composition.
   - The components: `icon_button`, `panel_header`, `toolbar`, `segmented`, `overflow_menu`, `section_header`, `list_row`, `empty_state`, `banner`, `status_chip`, plus the `prose_editor_state` constructor.
   - The `notice_tone` helpers with the `notice_tone_for` guard and the outcome mapping; `last_work_notice` holds `(Tone, String)`.
   - The pure functions `panel_widths` and `margin_mode`.
   - Retire the bordered `panel()` look in favor of sections and hairlines.
   - The keyboard menu test (Tab to ⋯, ↵, ↓, ↵).

   **Done when:**
   - all glyph controls use icons with labels
   - no nested bordered cards remain
   - error notices render as error banners and Stale outcomes as warnings
   - the contrast tests cover the new roles and the danger fill
2. **Shell and panel chrome** (`threads_shell.rs`, `threads_tiles.rs::tile_controls`, `threads_layout.rs` labels, `threads_lists.rs`).
   - **Title bar:** brn (Home) · Search ⌘K · needs-you badge · Write/Chat/Solo as one `segmented` control (IDs `layout-write`, `layout-chat`, `layout-solo` and their tooltips kept).
   - **Navigation bar:** Chats, Notes, Projects, Tasks · Changes · Settings icon · Open beside.
   - **Status bar:**
     - Run state and Stop (`stop-work`).
     - `status-save`: "● Unsaved · ⌘S" (warning; click or ⌘S saves) while the open note has unsaved changes, and "Saved" (muted; tooltip "Pilot proposal v1 saved · Open Changes"; click opens Changes) otherwise. It stays when the Note panel is hidden, so the open note's unsaved state is always visible. Recoverable edits for other notes stay listed on Home.
     - `status-layout` menu, labelled "Layout", or "Maximized · Note" while a panel is maximized. It contains:
       - the Write/Chat/Solo presets (⌥1–3, current one checked)
       - the panel toggles
       - Last work status
     - Keys (`status-keys`, Keyboard icon).
   - **Theme:** the `status-theme` pill (`threads_shell.rs` 918) is removed, and the status bar has no theme control (open question 4). Theme switching lives in Settings → General, through the same `choose_theme` with running-work deferral; ⌘K keeps its theme commands. Settings' "Also in the status bar and the launcher (⌘K)." (`threads_tiles.rs` 1109) becomes "Also in ⌘K."
   - **Contract correction:** the owner decision supersedes the [Omakase spec](../omakase-ui/spec.md)'s "theme menu" status-bar item (line 12) and "Selectable from the status bar, launcher and Settings › General" (line 17). Implementation annotates both lines with the decision and this plan's link.
   - **Panels:** sentence-case headers with icons and the single header ⋯.
   - **Launcher:** sizing and grouping; add the Projects command. `Command::label` names destinations instead of "Open tile: …": Notes, Connections, All chats, Chats, Current note, Current chat, Linked work and Projects (Changes, Tasks, Settings and Home already read so). Launcher tests that match "Open tile:" are updated to these labels.
   - **Keys help (`KEYS`):**
     - "Tile" becomes "Panel"
     - Maximize: "⌘⇧↵ · double-click its header · Maximize button"
     - Panel menu: "⋯ in the header · right-click the header"
     - Theme: "Settings → General · ⌘K"
     - Rewrite with AI: "⌘⇧R · writing bar · ⋯ menu"
     - Stop: "⌘. · status bar · Stop button"
     - add Menus: "Tab to ⋯ · ↵ opens · ↑↓ ↵ · Esc"
     - add Dialogs: "↵ submits a one-field dialog · ⌘↵ submits a form · Space presses the focused button · Esc cancels"
   - **Test updates:**
     - The theme journey (`threads_omakase_tests.rs`, from line 273) drops its `status-theme` steps and keeps the ⌘K and Settings → General switches with their saved-setting and running-work deferral assertions. It asserts that `status-theme` is absent and that the `status-layout` menu has no theme item.
     - The tiled-shell ID list (`threads_tiled_tests.rs` 491) drops `status-theme`.
     - Layout-menu tests choose "Chats" (sentence case) and assert the maximized label.
     - ⌘K lists Projects.

   **Done when** each visible control appears once, theme is reachable from Settings → General and ⌘K with no status-bar theme control, and the save state lives in the Note panel plus the single status-bar `status-save` indicator.
3. **Note tile: writing and review** (`threads_writing.rs`, `threads_workbench.rs`, `threads_document.rs`, `threads_diff.rs`).
   - **Toolbar:** one row holding the Write | Read `segmented` control (`note-write` / `note-read`), the formatting controls (Write only), Outline (`note-outline`, both modes; replaces the Write section row), the status chip (`note-status-chip`, "Protected · vN" or "Working · vN" in every state; component 9), the comment toggle with count (`toggle-margin`), Edit note (`edit-note-context`) and Connections (`note-connections`), collapsing as in the small-window rules. Read's "Select text to comment" hint becomes the comment toggle's tooltip.
   - **Note ⋯** keeps today's items, reordered so "Discard edit…" comes last after a separator and opens the Discard dialog.
   - **Banners:** recoverable edit, stale edit (with its preview) or newer version, and import summary, as in component 8. Below them, the selection strip while a passage is selected (layout rules).
   - **Writing bar** (IDs as today unless noted). The Stop line (progress plus `stop-writing`, outline) shows whenever this note's rewrite runs, independent of the rows below. The primary comes from the **first matching row**; controls from lower matching rows still render, as outline or ghost, never filled, and each control renders once:

| Order | State | Primary | Also visible |
|---|---|---|---|
| 1 | Dirty, note changed since the edit began | Save as new note (`save-buffer-copy`), rendered once, in the stale-edit banner | in the bar: row 2's `note-unsaved-chip` label and Discard…, Save (`save-note`) as a ghost (a refusal is a warning notice as today); no `save-buffer-copy` |
| 2 | Dirty | Save ⌘S (`save-note`) | "Unsaved edit · recoverable" (`note-unsaved-chip`, warning), Discard… (`discard-note`, opens the Discard dialog), Save as new note (`save-buffer-copy`); also in ⋯ |
| 3 | Proposal ready | Review rewrite (`writing-review-{op}`), newest proposal only; older ones outline | Dismiss; Rewrite with AI (`rewrite-note`) as ghost |
| 4 | Applied | Mark K comments addressed (`confirm-addressed`); none when K = 0 | "Applied · v4" (`applied-state`), Undo (`undo-writing-apply`), Rewrite again (`rewrite-again`, ghost), Hide (`clear-applied`, Close icon, tooltip "Hide; Changes keeps it and its Undo") |
| 5 | Last rewrite Failed | Retry saved rewrite (`retry-rewrite`) | error line "Rewrite failed · {run.progress}" |
| 5 | Last rewrite Interrupted or Cancelled | Retry saved rewrite | warning line "Rewrite interrupted · {run.progress}" or "Rewrite stopped · {run.progress}" |
| 5 | Last rewrite Working, no live worker | Retry saved rewrite | warning line "This rewrite may still be running elsewhere. Retry starts a new attempt." |
| 5 | Saved inputs, no run recorded | Retry saved rewrite | neutral line "Rewrite inputs saved" |
| 6 | Last rewrite Completed, no proposal | Rewrite with AI | neutral line "Rewrite finished · no proposal" |
| 7 | Idle | Rewrite with AI ⌘⇧R (`rewrite-note`) | scope summary (`rewrite-scope`), Retry saved rewrite (`retry-rewrite`), Undo previous rewrite (`undo-writing-apply`), saved inputs (`inspect-rewrite-inputs`, Info icon) |

   - Rows 5 and 6 replace today's `writing_saved_status` strings, under the same conditions (no live worker, no proposal, not applied). Their line (`writing-run-status`, new ID) sits above the bar on one line, truncated, with the full text in the tooltip and accessible label; `· {run.progress}` is omitted when empty. In rows 5, Rewrite with AI is a ghost.
   - The `writing-result` reply (120 px scroller) renders under that line whenever it does today, including a partial reply after a failed run.
   - **Blocked:** whichever primary a row selects stays in place, disabled, with its reason inline ("BRN is working", "Feedback couldn't load", "Exclude or re-select unlocated comments").

   - **Review bar** (IDs as today unless noted). Every state shows a footer (never clamped or truncated; Long content) naming exactly what Apply binds, plus the open-comments clause ("K comments stay open until you mark them addressed.", omitted when K = 0). The footer's verb is "Approving" when any note the proposal writes is protected, and "Applying" otherwise:

| Proposal shape | Footer |
|---|---|
| One note (whole note, editing final text, group selection) | Applying saves exactly this text. / Approving saves exactly this text to a protected note. |
| Several notes | Applying saves exactly these N notes. / Approving saves exactly these N notes, including protected notes. |
| Non-note proposal | Applying saves exactly these N changes. / Approving saves exactly these N changes, including a protected note. |

   During review the bar replaces the toolbar; its Proposal label plus the Protected chip is the Reviewing status. The Note ⋯ during review holds Compare side by side, Edit final text and Inputs, then Keep for later and Dismiss below about 920 pt (Small-window rules → Review bar). The bar, by state:

| State | Bar | Primary |
|---|---|---|
| Whole note, ordinary | Proposal · whole note · +a −r · Changes \| Result (`review-show-changes` / `review-read-result`) · Keep for later (`close-review`) · Dismiss (`dismiss-proposal`) | Apply whole note (`apply-final`) |
| Whole note, protected | same, plus the Protected chip | **Approve whole note** (`apply-final`) |
| Editing final text | Changes \| Result disabled · Cancel edit (`cancel-final`) | Use this text (`prepare-final`), disabled until edited ("Edit the text first"); Apply disabled ("Finish or cancel the edit first") |
| Group selection, partial | Include/Omit toggles (`review-group-*`) visible in the details, never in ⋯ | Prepare selection (`prepare-selected`); Apply disabled ("Prepare your selection first") |
| Group selection, all included | toggles visible; Prepare selection disabled | Apply / Approve |
| Shared changes that can't be separated | toggles disabled; visible line "These changes can't be separated. Apply all or ask for a new rewrite." | Apply / Approve |
| Several notes | `review-member-*` switcher visible ("Showing: A · B") | Apply N notes, or Approve N notes when any member is protected |
| Non-note proposal | Proposal · N changes · reason; details always visible | Apply N changes, or Approve N changes when a written note is protected |

   After Apply, the bar shows the "Applied v4" chip, Mark K comments addressed and Undo.
   - **Discard dialog** (destructive class). "Discard…" in the writing bar and "Discard edit…" in the Note ⋯ open it. Title "Discard your unsaved edit?"; body "Your changes since v{base version} will be lost. This can't be undone."; footer [Cancel] (`discard-note-cancel`) and a `danger()` [Discard] (`discard-note-confirm`) that calls today's `discard`. `store.discard` closes the session with no receipt or Undo, which is why it confirms. Tests that click `discard-note` then click `discard-note-confirm`.
   - **Editors:** every prose editor hides the line-number gutter: the note editor, the diff editor, `review_editor` (Edit final text), the Edit note instruction, the Save as decision text and the project instructions. A `prose_editor_state(window, cx)` constructor in `threads_components.rs` creates `EditorState` with `line_number(false)`, and those six sites use it. The composer, comment input and overall feedback already do this. Apply the 680 px measure.
   - **Comments:** the Wide, Narrow and Stacked modes (Layout rules), with `comments-section` placed in both Write and review.
   - **Contract correction** (owner decision, open question 2): the [writing polish spec](../writing-polish/spec.md) line 7 becomes "Comment cards never move into chat. They stay beside the note when the Note tile has room; in a narrow Note tile they move below the text into the in-tile comments section (owner decision, UI polish)." In the [target](../../../architecture/threads-target.md), "margin cards" (line 388 since the base move) and "passage comments beside the document" (line 390) gain "beside the document when there is room, otherwise in an in-tile comments section".

   **Done when:**
   - every combination of writing states has at most one filled primary, by the first-match order, and each writing-run outcome shows its tone and recorded reason
   - the toolbar is one row at 900×600, 1050×700 and 1250×860 in Write, dirty or not
   - at 900×600 in Write, the review bar fits one row, Protected stays visible, and the review footer is fully visible
   - in Stacked mode every comment capability works in `comments-section`, and no comment card renders in Chat
   - the four banners (recoverable edit, stale edit with its preview, newer version, import summary) stay visible in Write at 900×600
   - content gets at least 70 % of the Note tile's height at 1250×860, measured in Write in the idle state with a note of 24 headings, no banners, no selection and no review (the section row no longer exists, so headings add no chrome)
4. **Chat tile** (`threads_writing.rs` transcript and composer, `threads_chat.rs`, `threads_context_actions.rs`).
   - **Transcript:** 680 px measure; "You" and "BRN" labels; Copy as an icon; "Save as decision…" as a small ghost on completed replies.
   - **Cards:** the recovery card takes the strongest tone among its lines (below), and a needs-you question renders as a warning card. Only the newest card awaiting the owner is primary.
   - **Recovery card.** A template filled from recorded state, never a fixed cause. It shows when `can_continue_thread` holds **and `continue_path` is not `RetryRewrite`**, which is today's `latest_writing_instruction` gate. A writing continuation never renders in chat; the Note tile's Retry saved rewrite owns it (step 3).
     - **Path.** A pure helper `threads_state::continue_path(queue, messages, thread) -> ContinuePath` returns `ResumeQueue`, `RetryRewrite(request id)` or `FreshRun`, using the same checks `continue_run` makes today (the pending item's chat, then the newest writing request after the last owner message). `continue_run` dispatches on it, and the card uses it for its gate and label, so they cannot disagree. `RetryRewrite` exists only for that dispatch.
     - **Line 1, state and reason.** Line 1 uses the latest run's state only when no owner message postdates that run, by the receipt-order loop in `can_continue_thread`, factored into a helper both call (`can_continue_thread` returns before that loop for a continuable run). Otherwise it reads "Not answered yet" (neutral) with no reason, whatever the older run's state. The state word is followed by the run's recorded reason (`run.progress`) verbatim, on its own line that wraps in full and is never truncated (impl-plan.md Added scope U6, which supersedes the earlier one-line truncation with the full text in the tooltip); the accessible label carries the full text. The reason is omitted when `progress` is empty, and BRN never supplies a cause of its own.

| Recorded state | Word | Line tone |
|---|---|---|
| Failed | Failed | error |
| Interrupted | Interrupted | warning |
| Cancelled | Stopped | warning |
| Working (saved, no live worker) | May still be running | warning |
| no run yet, or an owner message newer than the latest run | Not answered yet | neutral |

   Shot 29 then reads "Failed · Provider unavailable (synthetic failure)". The card's tone is the strongest among its lines: error, then warning, then neutral.
     - **Line 2, what the button does**, by path and queue state. N = 1 (the pending item) plus the queued items; M = those from other chats; W = queued items while `paused`. Counts use "message" for 1 and "messages" otherwise.

| Path | Line 2 | Button |
|---|---|---|
| `ResumeQueue` | "Your message is saved and the queue is paused. Resume queue runs N waiting messages in order, including M from other chats." The paused clause appears only when `paused` is set; the "including" clause is omitted when M = 0. | **Resume queue** |
| `ResumeQueue`, pending item's run still Working | warning: "Your message is saved and the queue is paused. Resume queue stops the reply that may still be running, then runs N waiting messages in order, including M from other chats." Same clauses as above. Contract correction (S5 review S5-R1): `resume_chat_queue` stops that run before it resumes, and the card decides with that same check. | **Resume queue** |
| `FreshRun`, run still Working | warning: "This reply may still be running elsewhere. Continue starts a fresh reply and replaces it." When line 1 reads Not answered yet: "An earlier reply may still be running elsewhere. Continue starts a fresh reply and replaces it." | **Continue** |
| `FreshRun`, otherwise | "Your message is saved. Continue starts a fresh reply with the saved chat and current notes." | **Continue** |

   When `paused` is set, W > 0 and the path is not `ResumeQueue`, a further line follows: "W waiting messages stay paused until you resume the queue." (`continue_run` starts a fresh run, completion never unpauses, and the queue panel holds Resume queue.) Shot 29 shows this state: Continue, with "1 waiting message stays paused until you resume the queue."
   - **Queue notices** after Send, Queue or Save queued. N counts all waiting items ahead, including other chats. The rows are checked in this order and the first match wins, because Busy and Paused can hold together after Stop:

| Order | State | Notice | Tone |
|---|---|---|---|
| 1 | Edited item | Queued message updated. | info |
| 2 | Paused | Queued. The queue is paused; Resume queue to run it. | warning |
| 3 | Busy, N ahead | Queued; runs after N waiting messages. | info |
| 3 | Busy, nothing ahead | Queued; runs after the current reply. | info |
| 4 | Idle, not paused | none: the message dispatches at once and the run line shows Working. Any earlier notice is cleared. | — |

   - **Queue panel:** labels only items from other chats with their chat name, and shows the Paused tone with Resume queue.
   - **Archive** (`resolve-thread`) stays a one-click header control, because Unarchive reverses it and nothing is deleted. Its tooltip, accessible label and resulting notice state that it stops work and pauses the queue (Copy guidelines → Consequences).
   - **Archived chats:** the header shows the Archived chip. One composer banner, "Archived. Unarchive to reply.", holds the only Unarchive button, which keeps the `reopen-thread` ID. The body's `chat-unarchive` button and line are removed. Unarchive is the filled control and Send is outline while the chat is archived, because Send is refused there (S5 review S5-R4). Archive keeps the chat Open, so the recovery card can still show; it keeps line 1 and the recorded reason, line 2 reads "Unarchive to continue." (neutral), and `continue-*` is outline and disabled with "unarchive to continue" in its accessible label and tooltip, because the service refuses a run in an archived chat. Stop… does not depend on Archive (S5 review S5-R7).
   - **Steer** (`chat-steer`) keeps its place beside Queue while work runs, with the tooltip, label and notice from Copy guidelines → Consequences.
   - **Composer:** one row, with display names for the provider and the narrow-width model label.

   **Done when** shots 22, 23, 25, 29 and 40 each show one clear next action, the recorded failure reason and the paused and still-running consequences are stated in visible text, and each queue state shows its notice.
5. **Lists and workspace pages** (`threads_tiles.rs`, `threads_lists.rs`, `threads_connections.rs`, `threads_projects.rs`, `threads_search.rs`).
   - **Chats and Notes:** fixed rows; sentence-case sections; a one-row, horizontally scrollable A–Z strip that stays outside the scroller; the Topics prompt as one row ("Topics · Create Topics note").
   - **Home:**
     - The intro card shows only on first run, derived as no active notes and no active chats (no persistence change).
     - The pending delegation card (`delegation-pending`, placed by `delegation_host`) is independent of first run: it stays on Home whenever `delegation_host` returns Home and the choice is pending.
     - Rows instead of buttons, human times and "All changes".
     - Receipts that need refresh show the Needs refresh chip.
   - **Tasks:**
     - Sections instead of cards; empty groups hidden; Done and Cancelled collapsed with counts.
     - Each open task has exactly one visible main action, a compact outline button, never filled:
       - Suggested: "Accept task" (`open-action-*`)
       - Open: "Confirm done" (`done-action-*`)
       - Waiting: "Confirm done" (`done-action-*`)
     - The row ⋯ holds only the actions that change the task's state, with Cancel task last after a separator:
       - Suggested: Waiting, Confirm done, Cancel task
       - Open: Waiting, Cancel task
       - Waiting: Mark open, Cancel task
     - The no-op "Open" on Open tasks and "Waiting" on Waiting tasks are removed. Cancel task stays one click, because Changes can undo it.
   - **Changes:**
     - Rows show the reason, affected titles, the human time and an Undo ghost (Undo2 icon) that keeps `undo-{id}`, with the accessible label and tooltip "Undo: {reason}".
     - Receipts whose writes are only ChatControl collapse under "Background · N", still visible and undoable (open question 1).
     - Receipts with `needs_refresh` show a warning sub-row, "May need refresh", with the `refresh-record-{op}-{id}` buttons (the note titles) and "Open refresh follow-up" (`refresh-followup-{op}`). They never collapse under Background.
   - **Settings:** a horizontal `segmented` section switch (IDs `settings-section-*` kept) and forms in a 640 px column. Each section has at most one primary.
     - **Models:** two sections and one primary.
       - "Provider and model": provider and effort as `segmented` controls (`provider-*`, `effort-*` kept; effort labels Low, Medium, High), the model and credential inputs, the credentials line, and "Connect GitHub Copilot…" as an outline button.
       - "Helper sub-agents" (added on main in #145, owner-only, off by default): one line, "Read-only helpers can research your notes for the chat model. They can't change notes or stand in for an unavailable model."; the tested-models note (`HELPER_TESTED_NOTE`) as visible text, never hidden; the helper provider as a `segmented` control (`helper-provider-off`, `helper-provider-Codex`, `helper-provider-Copilot` kept); and when a helper is on, the helper model input, the helper effort as a `segmented` control (`helper-effort-*` kept) and "Uses the credential location above for its provider." Settings stays the only place to choose a helper.
       - Primary "Save settings" (`save-settings`) at the end of the section, with the visible line "Saves provider, model, credentials and helper settings." It covers exactly the fields that `configure` saves today, helper included.
     - **Delegation:**
       - Two standalone Kit `Radio`s carrying the existing `delegation-maintain` and `delegation-review-first` IDs, inside a container labelled "How should BRN update your notes?". Each Radio's accessibility label is its full option text. Neither is checked while the choice is pending.
       - A Kit `Switch` with the `maintenance-setting` ID shows Running / Paused and appears only after a choice.
       - Both still apply immediately through the existing `choose_delegation` and `pause` calls, with no Save.
       - `working_knowledge` also renders the pending card that `delegation_host` places on Home or in Chat. That card keeps two outline buttons with their one-line explanations and no filled primary. Both hosts keep the two IDs, and only one renders at a time, as today.
     - **Data:** primary "Create full backup"; "Change folder…" and "Export Markdown…" are outline buttons. `backup-directory` (the folder, or "Backup folder not chosen"), the line "Choose a folder covered by your separate backup system." and the `backup_status` and `export_status` lines stay visible.
   - **Projects:**
     - An Active | Trash `segmented` control replaces the `project-trash` toggle. The Trash segment keeps `project-trash`, and the Active segment gets `project-active` (new). Tests that clicked `project-trash` a second time to return click `project-active`.
     - **Active, no project selected:** "New project" (`project-new`) is the primary, beside the empty state or the list.
     - **Active, project selected:** the primary is "New chat" (`project-chat`), and New project becomes a ghost. The other project actions move into the Projects ⋯: Instructions & settings, New document, Import file…, Link document… and Move to Trash…. The status line "BRN knowledge: on/off · instructions" stays visible in the project header.
     - **Trash, project selected:** "Restore project" (`project-restore`) is the primary, and this line stays visible above it: "Restoring returns contents and settings. Cancelled tasks stay cancelled; tasks kept outside stay outside. Work stays paused."
     - **Trash, nothing selected:** the list with today's line "Deleted projects stay recoverable here." and no primary. When Trash is empty, the same line is the empty state's body, with no button.
     - The delete dialog follows the copy table and the destructive-dialog rules, with a `danger()` confirm (`project-delete-confirm`) and Cancel.
6. **States, small windows and focus pass** (all of the above).
   - Apply the Compact and Short rules, the panel-width modes, launcher and dialog sizing, the focus landing and the Escape order.
   - Give every dialog its ↵ class (`on_ok`), its `Dialog::footer` action row and, for multi-line forms, the ⌘↵ `Enter` capture plus `on_action::<Ask>`.
   - Audit every empty, loading (`Spinner` with "Preparing search…"), error, disabled-with-reason and success state against the components.
   - Re-observe natively in light and dark at 1250×860, 1050×700 and 900×600 using the seed fixture and harness, comparing against [`before/`](before/).

**Tests to add or update deliberately:**
- **Theme:** the five-theme contrast matrix from the color roles: danger fill and hover; each tone on tile, alt, paper and its tint; `ink` on selection, hover and each tint; `muted` on tile, alt, paper and hover.
- **Lists:**
  - a needs-you title stays visible beside a long reason
  - a selected row renders its metadata in `ink` and reports `aria_selected`
- **Notices:**
  - an `Err` arm and an `if let Err` site (`resume_chat_queue`) each render as an error banner
  - `WorkerEvent::Finished(Err("Synthetic failure remains visible"))` (the `threads_steering_tests.rs` fixture) renders an error banner, and Last work status restores it as an error
  - `Finished(Ok)` renders an error when the worker's `run_id` is recorded Failed, a success when it is Completed, and a success for a worker without a run
  - an `ApplyOutcome::Stale` renders as a warning, and an Applied outcome with `needs_refresh` renders as a warning that names Changes
  - a direct `notice =` overwrite of an error renders Info
- **Escape:** with a worker running and nothing open, Escape leaves the run Working and the queue unpaused. With a menu open in Settings, the first Escape closes only the menu.
- **Comments:**
  - `panel_widths` / `margin_mode` at 900, 1050, 1100 and 1250 pt and in Solo give Stacked, Stacked, Narrow, Wide and Wide, with Chats and with Notes in the navigation area
  - at 900×600 and 1050×700 in Write, no comment card renders beside the text or in Chat, and `toggle-margin` expands `comments-section` below the text; at 1250×860 the cards render beside it
  - in Stacked mode: a selected passage plus ⌘/ opens the section with a focused draft; two passage comments both render; Save and comment works while dirty; an unlocated comment can be reattached; Show passage selects the passage with the section still open; Rewrite with AI's request includes every comment and the overall feedback
  - Escape inside the section collapses it and keeps its draft
- **Note toolbar:** one row in Write at 900×600, 1050×700 and 1250×860, dirty or not, with Chats and with Notes in the navigation area.
- **Note at 900×600:**
  - Write with Chats and with Notes in the navigation area: no horizontal overflow and Stacked comments
  - Format ▾ holds the five actions, and Outline lists the headings in Write and in Read
  - the recoverable-edit, stale-edit (with `note-stale-preview`), newer-version and import-summary banners are each visible in Write
  - a dirty protected note keeps `note-status-chip` reporting "Protected · vN", and `note-unsaved-chip` is visible in the writing bar
- **Note sections:** choosing a heading from `note-outline` in Write reveals it (replaces the `section-0` assertion).
- **Selection strip:** with a selection, `selection-toolbar` is visible above the text; while dirty it shows Save and comment and `selection-comment-reason`.
- **Review:**
  - protected shows Approve whole note
  - editing makes `prepare-final` the primary (disabled until edited) and disables Apply
  - a partial selection makes `prepare-selected` the primary and disables Apply
  - the footer is visible in every state and reads "these N notes" for a several-note proposal and "these N changes" for a non-note proposal
  - at 900×600 in Write on a protected note: the bar fits one row, Approve is visible, Keep for later and Dismiss are in the Note ⋯, and the footer is fully visible
- **Writing bar:**
  - each state has at most one filled primary; in the stale-edit state Save as new note is the primary in the banner, and `save-buffer-copy` renders exactly once
  - dirty with a proposal ready: Save is the only filled primary, and Review rewrite is outline
  - dirty while generating: Save and Stop are both visible
  - a failed writing run shows `writing-run-status` in the error tone with its `progress` text, and Retry saved rewrite is the only filled primary
- **Discard:** `discard-note` opens the dialog and the edit is still open; ↵ leaves the dialog open; Cancel keeps the edit; `discard-note-confirm` discards it. The Note ⋯ "Discard edit…" opens the same dialog.
- **Composer:**
  - one row at 900×600 when idle
  - at narrow chat widths the model label includes the effort, with the accessible label "Model …, effort …"
  - while work runs, `ask-delegate` reads Queue at a wide chat width too
- **Tasks:** each open task has exactly one visible main action, never filled; the row ⋯ lists only the state-changing items for that state (no Open on Open tasks, no Waiting on Waiting tasks).
- **Changes:** ChatControl-only receipts group without losing Undo; needs-refresh receipts never group and keep `refresh-record-*` and `refresh-followup-*`; each `undo-{id}` has the accessible label "Undo: {reason}".
- **Chat:**
  - an archived chat shows one Unarchive (`reopen-thread`)
  - the queue notice matches each state, with Paused winning over Busy after Stop, and an Idle send clears an earlier notice
  - `continue-{id}` reads Resume queue when this chat holds the pending item, and the still-Working warning line is visible
  - a Failed run whose `progress` is "Provider unavailable (synthetic failure)" shows that text in the card
  - the shot-29 state (Failed, no pending item, paused, one queued message) shows Continue and "1 waiting message stays paused until you resume the queue."
  - a Completed run followed by a newer owner message shows "Not answered yet" and the `FreshRun` line 2; with a still-Working run instead, the card is warning and reads "An earlier reply may still be running elsewhere…"
  - a linked chat whose newest request is a writing request renders no `continue-*` card, and the Note tile shows `retry-rewrite` (keeps the `threads_steering_tests.rs` behavior)
- **Keyboard:**
  - Tab to a ⋯ button, ↵ opens it, ↓ ↵ activates an item
  - ⌘K lists Projects, and destination commands read "Notes", "All chats" and so on, never "Open tile:"
  - after opening each destination, focus lies inside its panel; for Chats (Recent) `threads_query` is not focused, and for All chats it is
- **Dialogs:**
  - New note: typing a title and pressing ↵ creates the note; ↵ with an empty title keeps the dialog open
  - Edit note: ↵ in the instruction inserts a newline, and ⌘↵ with focus in the editor submits an instruction with no added newline
  - Save as decision: ↵ in the title leaves the dialog open, and ⌘↵ saves
- **Destructive dialog (project Delete):** ↵ leaves the dialog open and moves nothing to Trash; Escape closes it; Tab to `project-delete-confirm` and Space moves the project to Trash when no task choice is pending.
- **Projects:** with no project selected New project is the primary; in Trash with a project selected Restore project is the primary and the restore line is visible; `project-active` returns from Trash.
- **Panels:** an Open beside panel shows the Split chip.
- **Delegation:** Settings renders the two Radios and the Switch; the pending card on Home renders two outline buttons and no filled primary; both use `delegation-maintain` and `delegation-review-first`.
- **Theme:** the journey switches through Settings → General and ⌘K with the saved setting and running-work deferral; `status-theme` is absent and the Layout menu has no theme item.
- **Unchanged flows:** the review primary, Changes | Result focus, and the Apply/Undo flows.

Tests that assert retired lowercase titles, glyph labels, button rows or moved control IDs are updated to the new labels and to menu items chosen by label, never weakened. Verify with `scripts/verify-desktop-shell.sh` (serial, `RUST_TEST_THREADS=1`, per the precedent dialog-timing flake), `cargo fmt --all -- --check`, `python3 scripts/check-markdown-links.py` and `git diff --check`.

## Out of scope

- New capabilities:
  - a desktop backup Restore UI
  - a native macOS menu bar
  - arrow-key list navigation
  - multiple note editors
  - editable WYSIWYG
  - a floating selection toolbar (no selection bounds in 0.7.3)
  - Kit `Diff` for prose review
  - a reading-size preference
  - new themes, or changes to the five base palettes
  - animation
- Any change to checked services, schema, persistence, authority, provider/runtime guidance, intake, retrieval or the CLI. Collapsed sections, filters and the first-run test are session-only or derived.
- Dependency or toolkit bumps, IME qualification, other platforms, real user data, provider calls, publication, merge and release.

## Preserve

Refine these owner-accepted decisions; do not reverse them.

- **Panels:** ordinary navigation reuses the Navigation, Main and Companion areas; only explicit Open beside adds a split. Settings stays a dismissible surface, Solo stays reversible, panels stay resizable and reorderable, and the focused panel stays revealed.
- **Navigation and look:** Chats / Notes / Tasks (plus Projects) navigation, with Home on the logo and Changes and the Settings gear secondary. Chat browsing keeps Recent / All / Archived. System typography with the 12/14/16/18 roles, quieter controls, one focused-panel accent, and search and filter controls outside scrolling content.
- **Composer:** the compact chat composer, with the Quiet eye beside model and effort (effort stays visible at every width) and no line-number gutter.
- **Note tile:** every writing interaction stays in the Note tile: formatted Read, Write formatting controls, passage and overall comments, saved-selection comment continuation, document-local candidate review, Retry and Stop, proposal-only Apply, addressed confirmation and Undo. Chat stays a separate panel and writing never needs Chat Send. Comment cards stay beside the text when the Note tile has room (Wide, Narrow). In a narrow Note tile they move to the in-tile comments section. They never move into chat, and every comment capability is kept.
- **Chat:**
  - work: durable queue and Steer; Stop/Resume; failure and restart recovery, including the distinct Resume queue and Continue paths; saved drafts; Archive and Unarchive; streaming and Jump to latest
  - attachments: scoped attachments and Keep in BRN
  - display: Quiet on/off; restorable dismissed completion status (Layout → Last work status); no duplicated model controls
  - actions: contextual Edit note, and reviewed Save as decision
- **Workspace:**
  - the ⌘K launcher
  - the five themes (Sammal, Läänemeri, Raba, Virmalised, Paber), selectable from Settings → General and ⌘K, with persistence and running-work deferral
  - the status bar's run state and Stop, save state (click saves; Saved opens Changes), layout menu and Keys (no theme control, per open question 4)
  - Projects: instructions, knowledge setting and its visible status, documents, chat movement, the checked delete preview with Keep outside / Cancel task choices, and Trash restoration
  - Search, Topics, Connections and Knowledge check; Markdown export; full backup to the remembered folder with Change folder, with the folder, its backup-system line and the backup and export status visible
  - protected notes; Tasks (Actions) and Changes (history) with Undo and the needs-refresh follow-ups
- **Invariants:**
  - existing functionality keeps working. Controls that stay visible keep their IDs. Controls moved into menus are listed above and stay reachable by mouse and keyboard.
  - accessibility labels, plus ≥ 4.5:1 contrast for text, controls and tints
  - protected-note authority, shown as Approve on protected reviews
  - exact-candidate review (Apply binds the displayed text, and the untruncated footer says so in every state and at every width)
  - version checks and stale refusals, including the newer-version and stale-edit banners and the saved-version preview
  - replay and durable operation identity
  - recovery, including the recoverable-edit banner and the visible Continue/Resume consequences
  - Undo

  Every write still goes through the existing checked service. Important status and consequences stay visible: needs you, unsaved, changed since your edit began, protected (also while dirty), failed (with its recorded reason, in chat and in the Note tile), paused (including waiting messages that stay paused), archived, reviewing, partial coverage, needs refresh, still running elsewhere, and Stop.

## Open questions (conservative defaults apply)

1. **Background bookkeeping receipts. OWNER DECISION: left to design judgment; the default below is adopted.** Receipts that only write ChatControl (queue initialize and recovery, created on every launch) crowd Changes and Home. Stopping those receipts from being created at all would change the checked service and is out of scope here. *Adopted:* Changes shows them collapsed under "Background · N", still expandable and undoable; Home's recent changes excludes them. Nothing is hidden permanently, and receipts touching notes, tasks, projects or settings, or needing refresh, are never grouped.
2. **Comments in narrow windows. OWNER DECISION (this assignment):** comments do not have to stay beside the text at smaller widths. Keep them beside the text when there is comfortable room. When the Note tile is narrow, move them to whatever placement is easiest to use, for example below the text or a comments section/sheet the user opens inside the Note tile. Usability decides; strict side-by-side does not. They stay in the Note tile and never move into chat. Every comment capability stays intact: multiple passage comments, overall feedback, saved-selection continuation, reattach and the full-context rewrite. This corrects the [writing polish spec](../writing-polish/spec.md) line "Comment cards never move into chat or below the note" for narrow widths only.
3. **Restore discoverability. OWNER DECISION: accepted default.** No UI change in this pass. Restore stays the documented CLI command, and a desktop Restore flow would be a separate assignment.
4. **Theme in the status bar. OWNER DECISION (this assignment):** the status bar does not need theme switching. Remove both the separate `status-theme` pill and any Theme section in the status bar's Layout menu. Theme switching lives in Settings → General; ⌘K can keep its theme commands because they add no visible chrome. This corrects the [Omakase spec](../omakase-ui/spec.md) wording "Selectable from the status bar, launcher and Settings › General" and its "theme menu" status-bar item. Update the theme journey test and the Settings hint ("Also in ⌘K.").

## Gates

- Defined: owner request, 14 October 2026.
- Plan: this document. Design review rounds 1–4 returned CHANGES_REQUESTED (round 1: 5 HIGH, 14 MEDIUM, 12 NIT; round 2: 1 HIGH, 8 MEDIUM, 16 NIT; round 3: 1 HIGH, 6 MEDIUM, 10 NIT; round 4: 1 HIGH, 1 MEDIUM, 4 NIT); all findings are addressed below. Round 5 (0 HIGH, 1 MEDIUM, 4 NIT) was carried into the [implementation plan](impl-plan.md) and implemented in S2 and S4.
- Contract corrections (owner decisions, open questions 2 and 4): applied. Writing polish spec lines 7 and 15 and target lines 388 and 390 (S4); Omakase spec lines 12 and 17 (S2).
- Implemented: yes. Slices S1–S7 and one integration fix on `work/ui-polish` (`13d6224e`…`ed8daa82`). All seven slice reviews and the integration review are APPROVED, with non-blocking findings open ([evidence](evidence.md)).
- Verified (automated): yes, the full gate on `ed8daa82`: 245 native tests, 75 + 75 feature-lane tests, 27 default-feature tests, clippy, fmt, links and diff checks clean ([verification](verification.md)).
- Native-observed: partly, with real input on the product build at 900×600 and 1250×860 in light and dark themes. Five defect groups are recorded and not yet fixed, and running-worker states, U3 rows and several banners were not observed ([verification](verification.md)).
- Owner-accepted: no.
- Published / merged / released: no; not authorized. `docs/status.md`, `threads-behavior.md` and `invariants.md` wording, U2 and the watch-item reruns wait for integration with `work/daily-workflow-reliability`.

## Responses to the design reviews

### Round 4

All 6 findings are addressed; none were backlogged or declined. H1 and M1 apply the owner decisions in open questions 2 and 4, and they supersede the earlier answers that kept comments beside the text at every width (round 1 H2, round 2 N13, round 3 N9) and theme in the Layout menu (round 1 M1). N1 supersedes round 3 M4's "two lines at most". **Detail:** the target's comment wording sits at lines 384 and 386 in this worktree, not 388 and 391, and the Settings theme hint is at `threads_tiles.rs` 1109, not 1180; the plan cites the checked lines.

| Finding | Response |
|---|---|
| H1 Comments at narrow widths | Fixed. `margin_mode` gives Wide, Narrow or Stacked; Rail and its overlay are gone. Stacked docks `comments-section` between the text and the writing bar (and in review), capped at 40 % of the Note body, reusing the same renderers and IDs; `toggle-margin` is its only toggle, ⌘/ opens it with a draft, Escape collapses it. Worked numbers, scrollbar range (850–853 pt), Escape order, scrollers, Out of scope, Preserve, tests and the contract corrections are updated. **Detail:** the section starts collapsed in session state, so a small window keeps its text height until the owner opens comments; the count and highlights stay visible. |
| M1 Theme in the status bar | Fixed. `status-layout` has no Theme section; theme lives in Settings → General and ⌘K; the Settings hint, Keys help, Done when, theme test, tiled ID list, Preserve and the Omakase spec correction are updated. `status-theme` moves from the menu-only table to a Removed note. |
| N1 Footer clamp | Fixed: never clamped or truncated; two lines is the 900×600 fit target, wrapping further with the binding sentence first. |
| N2 Toolbar fit | Fixed: starting thresholds of about 760 pt (Format ▾), about 620 pt (Edit note, Connections) and 480 pt (icon triggers, short chip), set from measured widths in step 3; one-row tests at 900×600, 1050×700 and 1250×860. |
| N3 Mixed card tone | Fixed: the card takes its strongest line tone, and the Working line reads "An earlier reply…" under Not answered yet; tested. The receipt-order loop is factored into a helper, since `can_continue_thread` returns before it for a continuable run. |
| N4 Duplicate `save-buffer-copy` | Fixed: in row 1 the bar keeps the unsaved label and Discard…, shows Save as a ghost and omits `save-buffer-copy`; tested to render once. |

### Round 3

All 17 findings are addressed; none were backlogged or declined, and two fixes add a detail (marked **Detail**). H1 withdraws round 2's `RetryRewrite` card row, and N2 supersedes round 2 M2's `latest_run` tone source. N3 was checked against GPUI 0.3.9 `window.rs`: key bindings dispatch before key listeners, then capture, then bubble, so a bubble-phase Settings handler runs after any focused menu or popover.

| Finding | Response |
|---|---|
| H1 Rewrite retry in chat | Fixed. The card's gate is `can_continue_thread` and not `RetryRewrite` (today's gate); `RetryRewrite` is dispatch-only; its copy row is deleted; the test asserts no `continue-*` card and a visible `retry-rewrite`. |
| M1 ⌘↵ in editors | Fixed with `capture_action::<Enter>` on each multi-line dialog root (secondary, not shift → submit, stop propagation), keeping `on_action::<Ask>` for buttons. Test asserts no added newline. |
| M2 Overlapping bar states | Fixed. Stop is independent; the primary is first-match (Dirty-stale, Dirty, Proposal ready, Applied, run outcome, Idle); lower rows render outline or ghost. Dirty + proposal and dirty + generating are tested. |
| M3 Line 1 with a newer message | Fixed. Run state only when no owner message postdates the run; otherwise "Not answered yet", neutral, no reason. Tested with a Completed run. |
| M4 Review bar fit | Fixed. Collapse at 620 and 480 pt, footer never truncated (two lines at most), the bar replaces the toolbar and carries Reviewing. Tested at 900×600 on a protected note. **Detail:** the two moved controls are added to the menu-only table. |
| M5 Dirty chips | Fixed. `note-status-chip` always shows Protected or Working with vN (icon and vN below 480 pt); `note-unsaved-chip` moves to the writing bar label. Toolbar arithmetic updated to about 290 pt; tested. |
| M6 Writing-run outcomes | Fixed with order-5 and order-6 rows (`writing-run-status`, new ID), Retry as primary with Rewrite as ghost, and `writing-result` placed and listed as a scroller. **Detail:** a fourth order-5 row covers today's "Rewrite inputs saved" case (request with no run). |
| N1 Tone on selection | Fixed in the color roles: tone text never on `selection` or `active`. |
| N2 Finished(Ok) tone | Fixed: from `worker.run_id`'s recorded state; no run → success. |
| N3 Escape and menus | Fixed: Settings' Escape moves to a bubble-phase handler; tested. |
| N4 Delegation hosts | Fixed: Radios and Switch in Settings only; the pending card keeps outline buttons; IDs shared, one host at a time. |
| N5 Launcher archived | Fixed: Archived chip and "archived" in the accessible name. |
| N6 Import tone | Fixed: info only for recorded full coverage. |
| N7 Backup status | Fixed in Settings → Data and Preserve. |
| N8 Undo names | Fixed: `undo-{id}` with "Undo: {reason}". |
| N9 850 pt overflow | Fixed: scrollbar expected from 850 to 881 pt; 900×600 is the no-overflow bound. |
| N10 Header titles | Fixed: note title kept during review; "New chat about {note}". |

### Round 2

All 25 findings are addressed; none were backlogged or declined, and none required reversing an owner-accepted decision or exceeding Kit 0.7.3. The Kit and GPUI behaviors behind M1, N1 and N12 were rechecked in the registry sources (`gpui-base` `dialog.rs` binds `enter` to `Confirm`; `gpui-component` `Dialog::on_ok` defaults to closing and `Dialog::footer` exists; a single-line `Input` propagates Enter; GPUI 0.3.9 tries the next matching binding when an action propagates). Five fixes differ in detail from the reviewer's wording. Each is marked **Detail**.

| Finding | Response |
|---|---|
| H1 Failure card | Fixed. The recovery card is a template: line 1 is the recorded state word plus `run.progress` verbatim (truncated, full text in the tooltip; superseded: wraps in full, impl-plan.md Added scope U6); line 2 states what the button does per path; a further line names the W waiting messages that stay paused. Tests cover the shot-29 text and stays-paused line. **Detail:** a pure `continue_path` helper now decides the label and the action together. It adds a `RetryRewrite` row ("Continue retries the saved rewrite…"), because `continue_run` retries a writing request in that case, and "starts a fresh reply" would be false there. |
| M1 Kit dialog ↵ | Fixed with the dialog-class table: every dialog sets `on_ok`. Single-field dialogs submit through it (false on validation error). Multi-line forms, pick lists and destructive dialogs return false. Tests cover ↵ on the delete dialog, Escape, and Tab + Space. **Detail:** multi-line forms (Edit note, Save as decision, project instructions) also return false, so ↵ never submits work or a protected decision by accident. They submit with ⌘↵, click, or Space. |
| M2 Tone mapping | Fixed. The mapping covers `if let Err`, `map_err`, worker-disconnect, `Finished(Err)` and `Converted(Err)` sites. `Finished(Ok)` takes its tone from the chat's latest run state, and `last_work_notice` stores the tone. The steering fixture test is added. |
| M3 Stale-edit banner | Fixed. `note-stale-banner` (warning) has the reworded text, Save as new note as the panel's primary, and Open current version. Below it sits the 180 px `note-stale-preview` scroller. It is listed as a capped scroller, in Preserve and in the 900×600 test. **Detail:** no "Compare side by side" button, because `compare_tile` needs a review proposal and would show "No whole-note proposal to compare."; the inline preview is the comparison. When the newer-version condition also holds, only the stale-edit banner renders, carrying `show-current-note`. The writing bar gains a "Dirty, changed since the edit began" state. |
| M4 Chrome budget | Fixed. Write's section row folds into the shared Outline menu (`section-*` retired, test updated). The selection strip stays a contextual row above the text, outside the budget, keeping its IDs and restyled with toolbar tokens. **Detail:** to keep one toolbar row at 900×600 with Outline added, Format and Outline become icon-only triggers below 480 pt (Type and TableOfContents, both in `gpui-kit-assets` 0.7.3). |
| M5 Navigation width | Fixed with per-kind minimums: Chats, All chats and Notes 170; Note 360 + margin; Chat 300; Compare 320; others 260. The worked numbers hold for any navigation-area list. The 900×600 test runs with Notes in the navigation area. |
| M6 Chats focus | Fixed. Chats (Recent) focuses the panel body, then `focus_next` to `chats-recent`; All chats focuses `threads_query`. Both are tested. |
| M7 Selected-row contrast | Fixed. Selected rows render metadata in `ink`. `muted` is limited to tile, alt, paper and hover, and the test matrix covers those. Rows set `aria_selected`. **Detail:** the previous starting hues failed on light tints and alt, and dark danger failed on Raba's tint. They are replaced with hues checked by script against the theme's own `mix`: every tone ≥ 4.6:1 on tile, alt, paper and its tint. |
| M8 Rule conflicts | Fixed. The consequence rule is scoped to irreversible and recovery decision points. Archive, Steer and Stop keep tooltip, accessible label and notice, and the copy is given. "Destructive" is defined, and Discard becomes "Discard…" with a destructive dialog (writing bar and Note ⋯). Dismiss proposal and Cancel task stay one click and say why. |
| N1 Space and menus | Fixed: "Tab to it and ↵ opens it"; the Keys row was already "↵ opens". |
| N2 Empty-state buttons | Fixed: one [New note]; the body names Notes and ⌘K. |
| N3 Review footer | Fixed with the per-shape footer table, including the Approving variants. |
| N4 Applied + needs refresh | Fixed: warning tone, and the copy says "Open Changes" without "records". |
| N5 Queue precedence | Fixed: ordered table, first match wins; Idle clears an earlier notice. |
| N6 Editor gutters | Fixed: six prose editors use `prose_editor_state`, which sets `line_number(false)`. Project instructions was added. |
| N7 `auto_grow` | Fixed: `auto_grow(1, 4)` follows content; the 72 px budget is the single-line state. |
| N8 Projects states | Fixed: Active / Trash `segmented` (`project-trash` kept, `project-active` new), primaries per state, restore line visible. |
| N9 Split marker | Fixed: neutral "Split" chip with a tooltip for `Area::Beside`. |
| N10 Launcher labels | Fixed: destination labels, tests updated. |
| N11 Task row ⋯ | Fixed: items by state, with no no-ops. |
| N12 ⌘↵ and footers | Fixed: `on_action::<Ask>` on multi-line dialog content, and action rows in `Dialog::footer`. |
| N13 Rail overlay | Fixed: width `min(280, text width − 120)`. "Show passage" closes the overlay, then selects the passage. Scrolling horizontally into the uncovered part isn't possible, because lines wrap under the overlay. |
| N14 Capped scrollers | Fixed: `note-coverage-details` and `note-stale-preview` are added. |
| N15 Delegation card | Fixed: `delegation-pending` stays on Home independent of first run. |
| N16 Queue label | Fixed: "Queue" whenever work runs, at every width. |

### Round 1

All 31 findings are addressed; none were backlogged or declined. Four fixes differ in detail from the reviewer's wording. Each is noted below. This table records round 1 as answered then. Round 2 revised five of these answers, and the body of the plan reflects the revisions:
- H4: a fourth banner (stale edit) is added (round 2 M3)
- H5: the fixed failure copy is replaced by the recovery-card template, and the consequence rule is scoped (H1, M8)
- M6: the tone mapping is extended (M2)
- M11: the Chats focus target is corrected (M6)
- M12: ↵ is explicitly inert through `on_ok`, and Space alone presses a focused confirm (M1)

| Finding | Response |
|---|---|
| H1 Escape stops work | Fixed. Escape only closes the launcher, menu, dialog, comments overlay and Settings, and never stops or pauses work; test added. |
| H2 Comments below the text | Fixed. Comments stay beside at every width (Wide 280, Narrow 220, rail with in-tile overlay). Stacking below is open question 2 with no default and is listed out of scope. |
| H3 Review states | Fixed with the review-bar state table, including protected Approve, Edit final text, group selection, shared changes, several notes and non-note proposals; the exact-text footer shows in every state. The protected, editing and partial-selection states are tested. **Detail:** several-note and non-note proposals read "Apply N notes / N changes" and use Approve when any written note is protected. This is stricter than today's single-note check and is display only. |
| H4 Note banners | Fixed. Newer-version, recoverable-edit and import-summary banners are toned, sit outside the chrome budget, never go into ⋯, and are tested at 900×600; the 70 % criterion is measured without banners. `toggle-coverage` stays in the banner, and Coverage details stays in ⋯ as today. |
| H5 Continue vs Resume | Fixed with the recovery table and failure copy, and the guideline now keeps recovery, authority, deletion and running-work consequences visible. **Detail:** the plain Continue state also keeps a one-line visible consequence. |
| M1 Status-bar routes | Fixed. `status-save` keeps both states and its click-through to Changes, presets and Theme sit in the Layout menu, and the theme journey test runs through it. Only the separate theme pill is removed (open question 4, naming the Omakase lines). The saved-state route is kept, so it needs no open question. |
| M2 Kit sizes | Fixed with the Kit realization table: Small 24, `.compact().h(28)`, default 32, and `Tag::custom` at 12 px. |
| M3 Tone channels | Fixed with the channel mapping and the contrast tests. Filled warning, success and info buttons are forbidden. |
| M4 One-primary scope | Fixed. The rule is one primary per panel plus Send; rows use outline, and only the newest awaiting card or proposal is primary. The Tasks test was reworded. |
| M5 Writing-bar states | Fixed with the seven-state table; Save stays the visible primary while dirty. **Detail:** with a proposal ready, Rewrite with AI is demoted to a ghost instead of moving to ⋯, which keeps `rewrite-note` visible and still leaves one primary. |
| M6 Notice tones | Fixed with the `notice_tone_for` guard, the ApplyOutcome/SaveOutcome mapping, `last_work_notice: (Tone, String)` and tests. |
| M7 Enqueue copy | Fixed with the five-state notice table and a test for each state. |
| M8 Breakpoint gap | Fixed. Content modes come from `panel_widths` and `margin_mode` (worked numbers given), and 1000 pt now governs chrome only. Tests run at 900, 1050, 1100 and 1250. |
| M9 Toolbar fit | Fixed with the collapse order (Format ▾ below 620, Edit note and Connections into ⋯ below 480) and a one-row test at 900×600. |
| M10 Reachability | Fixed. Projects is added to ⌘K, the keyboard menu route is stated, a test is added, and the menu-only actions are listed. **Detail:** Kit menu items have no element IDs, so moved controls retire their IDs and tests choose by label. If the keyboard test fails in 0.7.3, the fallback is ⌘K commands for the menu-only actions. |
| M11 Focus targets | Fixed with the per-destination table. **Detail:** Kit 0.7.3 keeps button focus handles private, so the pages focus the panel body and then use `focus_next()` to reach the first control, and the test asserts focus lands inside the panel. |
| M12 Destructive dialog focus | Fixed. Destructive dialogs bind no ↵ and keep Kit's default focus, so ↵ on open does nothing; the confirm needs a click or a deliberate Tab. Tested. **Detail:** after the owner tabs to the confirm, Space or ↵ still activates it, so keyboard-only users can confirm. |
| M13 Needs refresh | Fixed. The warning sub-row keeps `refresh-record-*` and `refresh-followup-*`; these receipts never group, and Home shows the chip. |
| M14 Composer effort | Fixed. The combined "model · effort ▾" label has an effort menu section and an accessible label, with a test. It is triggered by chat panel width below 420 pt. |
| N1 Two ⋯ | Fixed: one ⋯ per panel, in its header, with a Panel section. |
| N2 Keys help | Fixed in the step 2 copy updates. |
| N3 Chip IDs | Fixed: `status_chip(id, tone, label)` wraps the Tag in a div that carries the ID and label. |
| N4 Radio group | Fixed: a labelled container plus a full label on each Radio. |
| N5 Scrollers | Fixed: the allowed capped scrollers are listed. |
| N6 Unsaved overclaim | Fixed: the claim now covers the open note, and Home lists other recoverable edits. |
| N7 Related count | Fixed: only unlisted records are counted, and only the kinds present are named. |
| N8 Versions | Fixed: versions are allowed where authority or a conflict depends on them. |
| N9 First run | Fixed: derived as no active notes and no active chats. |
| N10 Unarchive ID | Fixed: the composer banner keeps `reopen-thread`, the header shows the Archived chip, and `chat-unarchive` is removed. |
| N11 Disabled reasons | Fixed: the reason is appended to the accessibility label, and shown inline where it decides the next step. |
| N12 Loose ends | Fixed: the Raba critique bullet is replaced by the danger-fill rule, the redundant Short rule is deleted, and the Projects status line is preserved. |
