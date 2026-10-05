# Active work

New slices follow the frozen [target](../../architecture/overview.md#frozen-target), current [roadmap](../../roadmap.md) and [development workflow](../../development/workflow.md). No new product implementation is authorized by this index. Historical rows remain at their existing paths for evidence and outstanding qualification; their specifications, process headers, model assignments and attribution rules are historical, not a queue of tasks to resume. Current roadmap work uses one bounded plan per active outcome.

| Task | State | Records |
| --- | --- | --- |
| Independent V1 review corrections | Exact quote/body evidence and Rust conflict identity passed independent review and local gates; exact-head CI/integration next. Rewrite protection follows separately; ratified cleanup and record-shape/performance corrections hold lifecycle integration. | [Correction plan](architecture-review-corrections/plan.md) |
| Knowledge foundations | Stage 5 foundations integrated; native/live/model/owner qualification remains open. | [Plan](knowledge-foundations/plan.md) |
| Text/email Inbox | Stage 7 active: intake, processing, knowledge/Action drafts, supersession, conflicts and original-review/removal preview integrated. Recoverable lifecycle snapshot `7c4f668` is unmerged; architecture-review corrections, including A1 record shape and owner cleanup, precede lifecycle integration/native removal/Stage 8. | [Inbox plan](text-email-inbox/plan.md), [supersession](text-email-inbox/supersession-plan.md), [conflicts](text-email-inbox/conflicts-plan.md), [original-copy work](text-email-inbox/original-copy-plan.md) |

Integrated main is `48941d3` (PR60). The full V1 goal is active; these correction
dependencies hold the next lifecycle integration, native removal and Stage 8.
See [current status](../../status.md) and the [checkpoint](../../development/checkpoint.md)
for current evidence. Plans retain previous observations; they do not override
newly ratified owner decisions or authorize external actions.

## Historical work at retained paths

The following rows are historical evidence and qualification records. They are
not work to resume or rename.

| Task | Historical state | Records |
| --- | --- | --- |
| Rig-first architecture reset | Superseded on 2026-10-02 by the simple Rig-based notes app; not executed. | [Plan pack](rig-first-reset/plan.md), [evidence/gates](rig-first-reset/evidence.md), [specification](../../superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md) |
| Simple Rig-based notes app | Historical baseline: Steps 1–4 implemented and merged on `main@50f898a` through PR #14; all eight Step 4 tasks and the whole branch Opus-reviewed, including final fixes through `dccc24a`. Offline/default/native build checks passed. Native/live/user acceptance remains pending. Future Steps 5/6 are superseded by the current roadmap. | [Historical roadmap](simple-rig-notes/plan.md), [search plan](simple-rig-notes/search.md), [AI chat plan](simple-rig-notes/chat.md), [evidence](simple-rig-notes/evidence.md), [historical specification](../../superpowers/specs/2026-10-02-simple-rig-notes-design.md) |
| Safe local Markdown note editing | Historical implementation: merged on `main@6323e53` through PR #13; integrated/native UI checks recorded, native retrieval unavailable (missing `protoc`); native acceptance pending. Preserve existing behavior until the roadmap's verified replacement/removal. | [Plan](markdown-note-editing/plan.md), [evidence](markdown-note-editing/evidence.md), [earlier spec](../../superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md), [historical reset](../../superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md) |
| UI slice 2 (chat polish) and vault search tool | Historical brainstorm paused before design approval. Rig chat is now merged through PR #14; any renewed UI scope needs reconciliation and separate approval. No slice-2 implementation is claimed. | [Paused brainstorm](ui-slice-2-chat-polish/paused-brainstorm.md) |

The early CLI foundation and PR2 review fixes are recorded under [completed work](../completed/README.md). The current CLI is owner-operated and exposes full workflow authority; future external agents are read/propose unless the owner explicitly delegates more. See [current status](../../status.md) and the [roadmap](../../roadmap.md) for other outcomes.

List newly authorized planned work here with a descriptive task folder and one short plan. Keep results in that plan or use an evidence record when a durable handoff needs it, following the development workflow. Small changes need no new task folder. Do not copy completed tasks into active work solely because wider product qualification remains open.
