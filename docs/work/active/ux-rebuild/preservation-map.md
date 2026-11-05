# Capability-preservation map

The full row-level inventory (353 capabilities, entry point, service owner and target destination) is in [capability-inventory.md](capability-inventory.md). It was produced read-only at `ab455c6d` and is updated per slice with status and evidence. This page records the destination decisions the spec left open, the retired mechanisms and their preserved outcomes, and per-area status.

## Lead decisions for unmapped capabilities

These are routine placement decisions within the approved direction (spec §§4.1–4.2, 6). None changes service behavior.

| Capabilities | Destination |
|---|---|
| CAP-009–011, 033, 049–050, 287 ⌘K launcher and its commands, chats, notes and "In chats" results | Sidebar **Search** opens the same launcher as a centred search surface (⌘K kept). Result groups label type and scope; commands stay a result group. |
| CAP-008, 039, 137 Home overview and ⌘0 | Retired as a destination. Attention content moves to **Inbox → For you**; first-use content moves to the **New chat** start surface. ⌘0 opens New chat. Recovery copy points to Inbox. |
| CAP-016, 126 Notice banner and precedence | Kept as one quiet inline notice at the top of the primary workspace, same precedence rules. |
| CAP-017 Last work status | Per-conversation activity disclosure in the conversation header. |
| CAP-018 Keys and mouse help | Workspace menu → **Keyboard shortcuts**, plus a launcher command. |
| CAP-030 Compact window | Replaced by the width-based pane policy (spec §8). |
| CAP-001, 032, 045 CLI flags, window close, ⌘Q exit flow | Unchanged. |
| CAP-040, 167–172, 180 Comments | Document canvas: selection strip **Comment** (⌘/ kept), a **Comments** toolbar toggle opening a side sheet (or margin when wide), Escape closes the sheet. |
| CAP-080 Attach document to chat | Composer **+** (reference/intake) menu, drag-drop and paste kept. |
| CAP-082, 266 Add useful information | Composer **+** menu and **Inbox → Imports**; both use the same service path. |
| CAP-090, 091 Linked work and capture/rewrite transcript rows | Conversation header **Linked** disclosure; transcript rows stay in the conversation. |
| CAP-150 Protect / Make working note | Document **⋯** menu, with the protected state shown in the document header. |
| CAP-205–207 Capture useful outcomes | Conversation **⋯** menu and the completed-reply actions; proposals appear in Inbox For you. |
| CAP-226 Brief and references | Document **⋯** menu → Brief and references sheet. |
| CAP-252 Move chat to project | Sidebar chat context menu and conversation **⋯** menu. |
| CAP-301 First-use delegation card | New chat start surface while unchosen; Settings → Delegation always. |
| CAP-336–339 MCP exact-input review | Inline permission card in the conversation above the composer; Settings → MCP keeps configuration. |
| CAP-341–344, 051 Web search | Composer **+** menu → **Search the web…** and a launcher command; same dialog and keys. |

## Retired mechanisms and preserved outcomes

| Retired (CAP) | Preserved outcome and new home |
|---|---|
| Tile layout presets, panels menu, open beside, move/swap/close tiles, gap resize, focus cycling (CAP-002, 019–029) | One primary workspace with a collapsible sidebar (⌘\ kept), assistant pane beside documents, resizable split, ⌘[ / ⌘] move focus between sidebar, workspace and assistant. Maximize becomes collapse-sidebar plus close-assistant. |
| Nav pills (CAP-003–007) | Sidebar entries Library, Inbox, Projects, Recent chats; Recent changes in Library/Inbox menus. |

## Status by area

Status values: planned, implemented, verified (automated), native-observed. Evidence links go in the slice evidence files.

| Area | Slice | Status |
|---|---|---|
| Dependency (GPUI Kit 0.9.0) | S0 | in progress |
| Appearance | S1 | planned |
| Shell, Keyboard | S2 | planned |
| Chat, Queue | S3 | planned |
| Writing, Comments, Review, Outline | S4 | planned |
| Library, Projects, Search | S5 | planned |
| Inbox, Tasks, Settings, Data, Skills, MCP, Web, Recovery | S6 | planned |
