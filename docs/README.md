# Documentation index

Start with [AGENTS.md](../AGENTS.md), then [current status](status.md). Read only the references needed for the task.

| Need | Read |
| --- | --- |
| Understand the product or run it | [Root README](../README.md), [setup](development/setup.md) |
| Understand desired product behavior | [Product vision](product/BRN_PRODUCT_VISION.md): requirements and future capabilities, not a claim that all are implemented |
| Review the proposed product architecture | [Opus audit](audits/BRN_PRODUCT_ARCHITECTURE_AUDIT.md), [independent review](audits/BRN_PRODUCT_ARCHITECTURE_REVIEW.md): advisory findings and freeze recommendations, not implementation authorization |
| Operate a workspace headlessly as an agent | [brn CLI](../crates/brn/README.md): what `brn` is, how to run it, `--json` envelopes and the command reference; it shares the application workflow, it does not bypass it |
| Find implemented work and open gaps | [Status](status.md) |
| Choose future work | [Roadmap](roadmap.md) |
| Review the proposed Markdown-first direction | [Design note: finite AI workspaces](superpowers/specs/2026-09-30-markdown-first-ai-workspace-design.md) (direction approved; detailed defaults under review, not implemented) |
| Review safe local Markdown editing | [Earlier approved open/save/recovery design](superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md), [implementation plan](work/active/markdown-note-editing/plan.md), [qualification](work/active/markdown-note-editing/evidence.md) (implemented and merged via PR #13; native acceptance pending; reset saving changes remain future work) |
| Review the UI redesign | [Workspace shell design](superpowers/specs/2026-10-01-workspace-shell-design.md), [UI feature backlog](ui/feature-backlog.md), [shell decision](architecture/decisions/2026-10-01-workspace-shell.md) |
| Understand code ownership | [Architecture overview](architecture/overview.md), relevant crate README |
| Review the historical Rig-first reset | [Specification](superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md), [plan pack](work/active/rig-first-reset/plan.md), [evidence](work/active/rig-first-reset/evidence.md): superseded, not execution instructions; current contracts are in the [simple notes roadmap](work/active/simple-rig-notes/plan.md) |
| Find the original reset handoff or paused chat-polish ideas | [Historical reset handoff](architecture/brn-rig-first-architecture-reset.md), [paused brainstorm](work/active/ui-slice-2-chat-polish/paused-brainstorm.md); neither authorizes new implementation |
| Change storage, revisions, comments or retrieval | [Invariants](architecture/invariants.md), [completed task records](work/completed/README.md) |
| Change provider behavior | [Rig AI contract](../crates/brn-ai/README.md), [shared workflow](../crates/brn-workflow/README.md). The [standalone sidecar trial](../experiments/codex-app-server/PACKAGING.md) is historical evidence, not a product gate. |
| Choose and report checks | [Verification](development/verification.md) |
| Start or hand off a task | [Workflow](development/workflow.md), [active work](work/active/README.md) |
| Understand an architectural choice | [Decision index](architecture/decisions/README.md), [dependency record](architecture/dependencies.md) |
| Reproduce an experiment | Its README under [completed work references](work/completed/README.md) |

## Document ownership

- `status.md` is the current progress summary, scoped to a stated commit and date.
- `roadmap.md` defines future outcomes and prerequisites, not implementation authorization.
- Architecture documents explain current boundaries and enduring constraints. Dated decisions retain rationale and supersession history.
- Crate READMEs describe local responsibilities, interfaces and tests; Rust source remains the API definition.
- Active plans define scope and acceptance. Evidence records observations, commands and limitations.
- Completed records preserve historical results. Their old branch names, machine paths, model assignments and permissions are not current instructions.

When implementation and guidance disagree, inspect the code and evidence, identify whether this is a defect or stale documentation, and reconcile the discrepancy explicitly. Do not silently reinterpret an invariant to fit the implementation.
