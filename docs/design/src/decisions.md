# Design decisions

Concise record of design choices. **Status** is *Proposed* until the owner
records *Accepted*, *Rejected with observation* or *Explicitly deferred* in the
owner column. Add new decisions at the end; supersede rather than rewrite.

| ID | Decision | Status | Owner |
| --- | --- | --- | --- |
| D1 | Keep the refined-terminal identity; correct its typography | Proposed, implemented | — |
| D2 | Six semantic tones, add danger red | Proposed, implemented | — |
| D3 | Navigator groups Workspace, Chats and Review | Proposed, implemented | — |
| D4 | Surfaces open beside the chat, never replace it | Proposed (continues approved shell), implemented | — |
| D5 | Persistent decision bar for proposals | Proposed, implemented | — |
| D6 | Inbox shows waiting originals first | Proposed, implemented | — |
| D7 | Header for global state; bottom strip for events | Proposed, implemented | — |
| D8 | Counts only when known | Proposed, implemented | — |
| D9 | Today becomes the home surface | Proposed, future design | — |
| D10 | Claim chips and activity trail for provenance | Proposed, prototype | — |
| D11 | Grouped consequence review with one decision bar | Proposed, prototype | — |
| D12 | Projects, People, Graph as navigator surfaces | Proposed, prototype | — |
| D13 | Handbook in Markdown + mdBook; tokens generated from code | Proposed, implemented | — |
| D14 | Real-view headless captures for documentation | Proposed, implemented | — |
| D15 | Recorded times shown in UTC for now | Proposed, implemented; preference open | — |
| D16 | Minimal, ChatGPT-desktop-like chat; vault rail closed by default | Owner direction, implemented | Owner feedback 2026-10-07 |
| D17 | Model and thinking-effort pickers in the composer | Owner direction, implemented | Owner feedback 2026-10-07 |
| D18 | Select text and right-click to comment in proposals | Owner direction, implemented | Owner feedback 2026-10-07 |
| D19 | Keep the refined-terminal theme | Owner liked it (feedback 2026-10-07) | Liked; formal acceptance pending |
| D20 | Composer always at the bottom | Owner direction, implemented | Owner feedback 2026-10-08 |
| D21 | Writing review: compact header, prominent prose text, visible comment highlights with hover | Owner direction, implemented | Owner feedback 2026-10-08 |
| D22 | Terminal-style chat: three directions prototyped, Hybrid recommended | Decided by D23 | Owner chose Hybrid 2026-10-08 |
| D23 | Hybrid terminal chat | Owner direction, implemented (partial: no claim tags yet) | Owner choice 2026-10-08 |
| D24 | Consolidate everything that needs the owner (Dashboard, Inbox, Needs Review, Review list) — *One queue* | Owner choice, prototype only; not implemented | Owner chose One queue 2026-10-08 |
| D25 | Settings layout — *ChatGPT-style modal* | Owner choice, prototype only; not implemented | Owner chose ChatGPT-style modal 2026-10-08 |
| D26 | AI writing environment — margin comments with optional Changes and Sources toggles (option 5) | Owner direction, prototype only; not implemented | Owner asked for the combination 2026-10-08 |

## D1 Keep the refined-terminal identity, correct its typography

The approved 2026-09 handoff chose square panes, monospace chrome and an
editorial centre. The native build rendered *everything* in 12 pt Menlo, which
made labels, prose and identity indistinguishable. Keep the palette, square
shape and mono-for-identity; use the platform sans for chrome and reading.
Serif reading (handoff option) is deferred to a future reading mode.

## D2 Six semantic tones, add danger red

Info, Attention, AI, Success, Neutral existed; failures and overdue items had no
distinct colour. Added `red` (dark `#f2a2a2`, light `#a3323a`, both ≥ 6.8:1 on
paper). Every tone is paired with text.

## D3 Navigator groups Workspace, Chats and Review

The left rail mixed conversations, surfaces, creation commands and every
proposal ever made in one undifferentiated list. Grouping by job (where work
lives / what I talked about / what awaits my decision) matches the vision's
chat-first home with compact operational visibility (§25). Decided proposals
collapse by default; Activity holds the full history.

## D4 Surfaces open beside the chat

Continues the approved combined workspace. Chat stays available while reviewing
or reading so the owner can ask about what is open.

## D5 Persistent decision bar for proposals

Approve and Reject were mixed with nine other buttons at the end of a long
scroll. The bar names the exact version and keeps the decision reachable.

## D6 Inbox shows waiting originals first

Vision §7.1 step 1 is “show what is waiting”. The previous layout put two
analysis forms first.

## D7 Header for global state; bottom strip for events

The header shows phase as a badge and Stop; the last event sentence moved to a
26 pt bottom strip so it no longer competes with navigation.

## D8 Counts only when known

Navigator counts come from already loaded pages; no extra workflow calls, and an
absent count never implies zero.

## D9 Today becomes the home surface

Dashboard evolves into *Today* with suggested focus and reasons (§26). Chat
remains the default centre. Depends on a workflow ranking with explanations.

## D10 Claim chips and activity trail

Lightweight per-claim provenance (§21) and a one-line activity trail with
budget (§29, amendment) instead of footnotes or raw transcripts.

## D11 Grouped consequence review

Inbox and maintenance consequences are independent proposals reviewed together
with one selectable list and *Approve n selected…* (§23).

## D12 Projects, People, Graph as navigator surfaces

First-class concepts (§13, §14, §43) get navigator entries once their views exist.

## D13 Handbook tooling

One Markdown source in `docs/design/src`, rendered with mdBook 0.5.4 installed
into `target/design-tools` (no global change). Token tables and prototype CSS
are generated from `tokens.rs`; a unit test fails when they are stale.

## D14 Real-view headless captures

Screen Recording permission is unavailable to agents and visible windows steal
focus. A development-only `native-capture` feature renders the production view
tree with the toolkit's headless Metal renderer over a synthetic workspace. It
is visual evidence only.

## D15 Recorded times shown in UTC

The UI had raw millisecond values. UTC is exact and identical across clients;
local-time display is a preference to settle (see
[open preferences](open-questions.md)).

## D16 Minimal, ChatGPT-desktop-like chat

Owner feedback, 2026-10-07: “extremely busy… too much info… should resemble the
GPT desktop app… needs to be more minimal”. Changes: vault rail closed on first
run (header toggle opens it; a saved layout keeps your choice); collapsed rails
take no space; header shows only toggles and a phase badge when not Ready; the
status strip appears only for real notices; the empty chat centres a greeting and
the composer; questions are right-aligned bubbles, answers plain text with a
quiet model line; sidebar without the Workspace label, Draft badges or chat ages
(ages in tooltips); fewer explanatory sentences; recovery and advanced commands
behind *More options* unless they are needed; Inbox cleanup and Source
preparation appear only when relevant.

## D17 Model and thinking-effort pickers in the composer

Owner feedback: model and effort “should not be hidden in Settings”. The
composer shows two dropdowns (model from discovered models, Low / Medium / High
thinking). They send the same selection commands as Settings, so each request
still freezes its choice and BRN never switches silently. Connecting accounts
and discovering models stay in Settings (linked from the model menu).

## D18 Right-click to comment

Owner feedback: commenting should work by selecting text and right-clicking.
The proposed-text editor's right-click menu offers *Comment on selection…*,
*Copy* and *Select All*; the existing exact-selection comment flow is reused.

## D20 Composer always at the bottom

Owner feedback 2026-10-08, with a Codex desktop reference: the message box
belongs at the bottom from the start, not in the middle. An empty chat centres
only the greeting.

## D21 Writing review

Owner feedback 2026-10-08: highlighting was hard to see; the proposed text should
be more prominent; the upper part should be more compact; comments should be
visible in the text and readable on hover, like Word. Implemented:
- One compact header row: state badge, editable title, and a single identity
  line (`Create · path · vN`); the trust callout moved into the decision bar
  sentence; member chips only when a proposal has several members.
- The proposed text is the main element: it fills the pane, uses the reading
  font at 15 pt, with no line numbers or folding gutter.
- Every exactly anchored text comment is shown as an amber highlight; hovering
  it shows the comment (the editor's hover popup). Anchors whose quote no longer
  matches are not drawn (never guessed) and remain in the comment list below.
- Text selection uses a clearly visible cyan tint in both themes.

## D22 Terminal-style chat directions

Owner request 2026-10-08. [Prototype](prototypes.md#terminal-style-chat-directions):
**1 · Pure terminal** (all monospace, `❯` prompts, tool lines, `:commands`),
**2 · Hybrid** (prompt and metadata in mono, collapsible trace, proportional
answers with claim chips — *recommended*: terminal feel without losing reading
comfort for long answers), **3 · Command blocks** (each turn a block with
actions; `/` command menu). Not implemented; waiting for the owner's choice.

## D23 Hybrid terminal chat

Owner chose the Hybrid direction (2026-10-08). Implemented: questions as a
monospace `›` prompt line; a monospace trace line with left rule (live tool while
streaming, recorded duration when finished — never invented steps); answers in
the reading font at 15 pt; a monospace footer with the recorded provider · model ·
effort and an inline *save as note…* link; the composer with a `›` prompt,
monospace input and monospace model/thinking pickers. Not yet implemented
because the workflow does not provide the data: inline claim tags
(`vault`/`unknown`/`conflict`), per-step trace and token counts.

## D24 Consolidate what needs the owner

Owner feedback 2026-10-08: Dashboard, Inbox, Needs Review, the Review list and
Decided are all open issues for the user, and the sidebar shows too much.
Prototyped in [round 3 directions](prototypes.md#round-3-directions-sidebar-settings-ai-writing):
**1 · One queue** (*recommended*: one *Needs you* page grouped by what the owner
must do — Due, Decide, Sort, Check — with filter chips; the sidebar keeps New
chat, Search, Needs you, Chats, History and Settings; Decided and Activity move
to History), **2 · Counts only** (separate places, one row and a count each),
**3 · Threads** (Codex-like; a proposal stays under the chat that made it),
**4 · Attention pill** (chats only; the queue sits behind a header pill).
All options keep the existing Inbox, proposal and finding workflows; only the
navigation changes. **Owner chose 1 · One queue (2026-10-08).**

## D25 Settings layout

The current Settings dialog mixes layout +/− buttons, the data path and account
diagnostics in one scroll. Prototyped in [round 3 directions](prototypes.md#round-3-directions-sidebar-settings-ai-writing):
**1 · ChatGPT-style modal** (*recommended*: tabs General, AI & models, Accounts,
Search, Data & safety, About; one decision per row; width buttons replaced by
dragging the sidebar edges plus *Reset layout*), **2 · Mac settings window**,
**3 · Status first** (health tiles, then preferences), **4 · Searchable list**.
Every option states that BRN never falls back to another model or account, and
that "connected" means stored credentials, not live availability. Items marked
*planned* (interface language, token display) need owner decisions first.
**Owner chose 1 · ChatGPT-style modal (2026-10-08).**

## D26 AI writing environment

Builds on D21. Prototyped in [round 3 directions](prototypes.md#round-3-directions-sidebar-settings-ai-writing):
**1 · Track changes** (what the last Rewrite changed; Clean, Changes and previous-version views;
version stepper), **2 · Margin comments** (*recommended*: comments in the right
margin aligned with highlights, linked on hover, BRN's reply under each comment,
open/addressed state, *Rewrite with N comments*), **3 · Inline AI + sources**
(dotted source marks for vault, web, BRN's suggestion and unknown — vision §6
items 4–7; select text for Comment, Rewrite, Shorter or Ask; inline Keep or
Discard), **4 · Draft canvas** (document plus a draft conversation with Sources
and Versions tabs). Recommended combination: 2 with the source marks from 3.
**Owner direction (2026-10-08):** margin comments (2), plus Track changes (1)
and Sources (3) as optional toggles — prototyped as **5 · Combined**. Comments
are always visible; *± Changes since vN* (key C) and *⋯ Sources* (key S) are
off by default; *show change* on an addressed comment turns on Changes and
flashes the edit. Inline edits only change the draft; approval still covers the whole exact
version. Source marks need claim-level provenance from the workflow, which
does not exist yet (same gap as D23 claim tags).

## Earlier design material reconciled

| Source | Kept | Superseded |
| --- | --- | --- |
| BRN UI handoff (2026-09, `brn/brn-ui-handoff`) | Combined workspace, refined-terminal palette, square shape, Focus, Settings placement, protected Sources, exact approval, explicit context | “Review queue” as a separate top-level area (now *Review* group + Needs Review), Pi SDK assumptions, 144/158 pt rail defaults |
| Electron workbench (2026-07/08) | Status strip, approval cards with before/after, honest AI interaction, notes-first main pane | Eight admin-like top-level surfaces, Agents/Usage as primary navigation, terminal theme variants as user modes |
| Pre-Electron design (2026-03/06) | “Agents propose, human approves”, provenance-first language | `_Staging` folder vocabulary |

Unresolved contradictions are listed in [open preferences](open-questions.md).
