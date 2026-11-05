# BRN UX rebuild (BRN-UX-002) — implementation plan

Assigned by the owner on 5 November 2026. Design authority: [docs/product/ux-rebuild/](../../../product/ux-rebuild/README.md) (PR #247, revision 1.2). Canonical product and data contracts stay with the [Threads target](../../../architecture/threads-target.md), [behavior contract](../../../architecture/threads-behavior.md) and [invariants](../../../architecture/invariants.md).

## Baseline and ownership

- Integration branch `work/ux-rebuild-20261105`, started from main `e749a83e` plus the PR #247 documentation commits (`ab455c6d`).
- One integration lead owns this branch, shared interfaces (route/state types, semantic tokens, shell layout policy) and merges. Helpers work on their own `work/ux-s*` branches in isolated checkouts with explicit file ownership and evidence files under this folder.
- Models: Opus 5.5 (Kiro) for implementation slices, Haiku 5.5 for read-only inventory and small mechanical tasks, Sol 6.1 (Codex, xhigh) for independent review of the integrated result.
- Synthetic data only, in fresh explicit data directories. No real owner data, release or provider spend beyond normal tooling.

## Slices

Each slice keeps the app building and existing behavior tests meaningful. Tests that only enforce the retired tile mechanism are replaced together with the production path.

| Slice | Scope | Primary files | Depends on |
|---|---|---|---|
| S0 | GPUI Kit `=0.9.0` migration, lockfile, API fixes (spec §12.1) | both `Cargo.toml`, `Cargo.lock`, API call sites | — |
| S1 | Semantic tokens and System/Light/Dark appearance; live OS appearance; persistence independent of agent fencing; legacy palettes under Advanced | `brn-writing-ui/src/theme.rs`, `native.rs`, desktop settings/theme persistence | S0 |
| S2 | Shell: quiet sidebar (New chat, Search, Library, Inbox, Projects, Recent chats, workspace menu), route model replacing tiles as the primary navigation, responsive pane policy, modal host and focus | `threads_shell.rs`, `threads_layout.rs`, `threads_lists.rs`, `threads_components.rs`, root render in `threads_ui.rs` | S1 |
| S3 | Conversation: centred column, flat assistant prose, single composer with context chip, model controls, Send/Queue/Stop, queue edit/reorder, activity disclosure, permissions, errors | `threads_chat.rs`, `threads_run_activity.rs`, `threads_notice.rs`, `threads_context_actions.rs` | S2 |
| S4 | Document canvas: Read/Edit, save state, collapsible assistant, Outline (READ-01..05), shared document styles, comments sheet, Rewrite, in-context review, Apply/Discard/Undo, history | `brn-writing-ui/src/*`, `threads_writing.rs` | S2 |
| S5 | Library (Articles default, All files, topics, filters, related/sources/checks) and Project workspace | `threads_lists.rs` (library), `threads_projects.rs`, `threads_connections.rs`, `threads_search*.rs` | S2 |
| S6 | Inbox (For you / Imports / Tasks), tasks detail, Recent changes, Search surface, Settings sections, Data (export/backup/restore/Trash), Skills, MCP, web search, recovery/error states | `threads_inbox.rs`, `threads_documents.rs`, `threads_capture.rs`, `threads_tiles.rs`, `threads_restore.rs`, `threads_skills.rs`, `threads_mcp.rs`, `threads_web_search.rs` | S2 |
| S7 | Retire tile chrome and dead actions; update shortcuts, help and behavior docs | across desktop | S3–S6 |
| S8 | Qualification: existing gates, focused new checks, fixture import, 42 acceptance cases, native visual matrix, independent review and fixes | evidence in this folder | S7 |

S3–S6 run in parallel after S2 lands; the lead merges them in order S4, S3, S5, S6 to keep `threads_ui.rs` conflicts small.

## Records

- [Capability-preservation map](preservation-map.md)
- [Acceptance results](acceptance.md)
- Slice evidence: `s*-evidence.md` in this folder

Status words follow the repository rule: implemented, verified, native-observed, owner-accepted and merged are reported separately.
