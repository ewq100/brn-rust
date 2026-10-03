# Active work

| Task | State | Records |
| --- | --- | --- |
| Rig-first architecture reset | Superseded on 2026-10-02 by the simple Rig-based notes app; not executed. | [Plan pack](rig-first-reset/plan.md), [evidence/gates](rig-first-reset/evidence.md), [specification](../../superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md) |
| Simple Rig-based notes app | Steps 1–4 implemented and merged on `main@50f898a` through PR #14; all eight Step 4 tasks and the whole branch Opus-reviewed, including final fixes through `dccc24a`. Offline/default/native build checks passed. Native/live/user acceptance remains pending. Steps 5/6 are future work. | [Roadmap](simple-rig-notes/plan.md), [search plan](simple-rig-notes/search.md), [AI chat plan](simple-rig-notes/chat.md), [evidence](simple-rig-notes/evidence.md), [specification](../../superpowers/specs/2026-10-02-simple-rig-notes-design.md) |
| Safe local Markdown note editing | Implemented and merged on `main@6323e53` through PR #13; integrated/native UI checks recorded, native retrieval unavailable (missing `protoc`); native acceptance pending. The reset proposes future saving changes, not removal of current implementation. | [Plan](markdown-note-editing/plan.md), [evidence](markdown-note-editing/evidence.md), [earlier spec](../../superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md), [reset](../../superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md) |
| UI slice 2 (chat polish) and vault search tool | Historical brainstorm paused before design approval. Rig chat is now merged through PR #14; any renewed UI scope needs reconciliation and separate approval. No slice-2 implementation is claimed. | [Paused brainstorm](ui-slice-2-chat-polish/paused-brainstorm.md) |

The agent-facing CLI foundation and its PR #2 review fixes were implemented, verified and merged; both are recorded under [completed work](../completed/README.md). See [current status](../../status.md) and the [roadmap](../../roadmap.md) for other outcomes.

Add a descriptive task folder with `plan.md` and `evidence.md` when starting planned work, then list it here. Follow the [development workflow](../../development/workflow.md). Do not copy completed tasks into active work solely because wider product qualification remains open.
