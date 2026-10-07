# Product map and coverage

Each important area of the [product vision](../../product/BRN_PRODUCT_VISION.md)
mapped to the user goal it serves, its place in the experience, and its design
and implementation state. **Implementation** refers to the desktop UI on this
branch; backend qualification lives in [status](../../status.md). Verification
and acceptance are in the [verification record](verification.md).

Legend — Design: **D** designed in this book · **P** shown in the clickable
prototype · **–** placement only. Implementation: **Impl** usable in the app ·
**Partial** a narrower version exists · **None**.

| Area (vision §) | User goal | Place in the experience | Design | Implementation | Assumptions / dependencies |
| --- | --- | --- | --- | --- | --- |
| Chat sessions (§19) | Ask, resume, keep sessions separate | Centre chat; *Chats* in navigator | D, P | Impl (ask, resume, history labels) | Archive/Restore/Delete need P9 lifecycle |
| Answer provenance (§21) | See where each claim came from | Claim chips in answers and drafts | D, P | None (answers are plain text) | Needs structured citations from workflow |
| AI activity & budget (§29, amendment) | Follow and stop work; see spending | Header phase badge, trail under answer, budget meta | D, P | Partial (phase badge, Stop, tool name) | Budget/usage reporting from provider |
| Session outcome capture (§19.2) | Turn decisions into durable knowledge | Inline AI callout under answer → proposal | D, P | Partial (“Review as new note…”) | Outcome detection |
| Inbox intake (§7) | Bring material in, see what waits | *Inbox* surface: waiting → process → review | D, P | Impl for text/email/teams/Markdown/DOCX subset | Real EML, PDF, PPTX, URLs (P2/P5) |
| Grouped consequences (§23) | Approve related changes together | Review group with checkboxes + one decision bar | D, P | Partial (group approval dialog per proposal) | Group listing view |
| Visual material (§7.2) | Know images are kept and interpreted | Source review shows asset + separate interpretation | D | Partial (one PNG + annotation) | Broader visuals |
| Original cleanup (§7.3) | Remove copies safely | Separate explicit, recoverable step after approval | D, P | Impl | — |
| Proposals & review (§23, §4 amendment) | Review exactly what changes | Proposal view + persistent decision bar | D, P | Impl | — |
| Comments → Rewrite (§5, §6) | Iterate on drafts | Comments section + Rewrite in proposal | D, P | Impl | Live model qualification |
| Writing grounded documents (§6) | Draft with known vs proposed content | Claim labels in drafts | D, P | None | Structured claim kinds |
| Current / Source / History (§4, §9) | Read the right kind of knowledge | Scope control in vault; kind badges on documents | D | Impl | — |
| Needs Review (§10–11) | Handle conflicts, staleness quietly | *Needs Review* surface + nav count | D, P | Partial (identity/link findings, Inbox conflicts) | Staleness & maintenance (P4/P6) |
| Supersession & conflicts (§9) | Keep one current truth, keep history | Finding → proposed update with diff | D, P | Partial | P4 |
| Actions & Dashboard (§12, §25) | See and finish unfinished work | *Dashboard* (future *Today*) | D, P | Impl (filters, counts, Complete, follow-up) | — |
| Work planning (§26) | Decide what to focus on | *Today* suggested focus with reasons | D, P | None | Ranking explanation from workflow |
| Projects (§13) | One coherent picture of a project | Project view: approved summary + live context | D, P | None | P8 projections |
| People (§14) | What am I waiting for from Anna? | Person view; identity confirmation | D, P | None (identity findings only) | P8; identity evidence |
| Graph (§18, §43) | Explore how notes connect | *Graph* surface; derived links dashed | D, P | Partial (relationships list) | P8 view |
| Web research (§22) | Use the web with attribution | Web claim chips; capture proposals | D | None | P6 |
| Helpers (§28) | Faster bounded work | Shown in activity trail; Settings | D | None | P10 |
| Provider & model choice (§28) | Explicit model, never silent switch | Settings; composer meta; per-turn record | D, P | Impl | — |
| Failure & offline (§30–31) | Know what still works | Status strip, disabled AI actions with reason | D, P | Partial | — |
| Activity & recovery (§33, §42) | See and undo approved changes | *Activity* surface | D | Impl | — |
| Manual note editing (§24) | Small corrections | Note view with Save to Markdown | D | Impl | — |
| Vault organization & cleanup (§17, P11) | Tidy a messy vault | Review batches in proposal view | – | None | P11 |
| Working preferences (§37) | BRN learns approved style | Settings → Working preferences | D, P | None | P9 |
| Session lifecycle (§19.3) | Archive, restore, delete safely | *Chats* list with uncaptured-outcome warning | D, P | None | P9 |
| External agents / CLI (§1) | Headless use | Not a desktop surface | – | n/a | — |
| Packaging & setup (§39) | Understandable first run | Welcome checklist (vault, model) | D | Partial (welcome checklist) | P13 |

Areas explicitly out of V1 scope and therefore not designed: full XLSX and
standalone-image import, direct sending, connectors, notifications, sync,
mobile, collaboration (vision §47 and the 2026-10-07 amendment).
