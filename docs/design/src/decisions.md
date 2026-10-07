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

## Earlier design material reconciled

| Source | Kept | Superseded |
| --- | --- | --- |
| BRN UI handoff (2026-09, `brn/brn-ui-handoff`) | Combined workspace, refined-terminal palette, square shape, Focus, Settings placement, protected Sources, exact approval, explicit context | “Review queue” as a separate top-level area (now *Review* group + Needs Review), Pi SDK assumptions, 144/158 pt rail defaults |
| Electron workbench (2026-07/08) | Status strip, approval cards with before/after, honest AI interaction, notes-first main pane | Eight admin-like top-level surfaces, Agents/Usage as primary navigation, terminal theme variants as user modes |
| Pre-Electron design (2026-03/06) | “Agents propose, human approves”, provenance-first language | `_Staging` folder vocabulary |

Unresolved contradictions are listed in [open preferences](open-questions.md).
