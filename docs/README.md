# Documentation index

Start with [AGENTS.md](../AGENTS.md). The owner froze the reviewed product architecture on 2026-10-03 and accepted the concise [development workflow](development/workflow.md). Use [current status](status.md) when choosing work; read only task-relevant references.

| Need | Read |
| --- | --- |
| Understand the product or run it | [Root README](../README.md), [setup](development/setup.md) |
| Understand desired product behavior | [Product vision](product/BRN_PRODUCT_VISION.md): requirements and future capabilities, not a claim that all are implemented |
| Understand the frozen target | [Architecture overview](architecture/overview.md#frozen-target), [invariants](architecture/invariants.md#frozen-target-guarantees). [Opus audit](audits/BRN_PRODUCT_ARCHITECTURE_AUDIT.md) and [independent review](audits/BRN_PRODUCT_ARCHITECTURE_REVIEW.md) are dated rationale, not execution plans |
| Operate a workspace headlessly as an agent | [brn CLI](../crates/brn/README.md): what `brn` is, how to run it, `--json` envelopes and the command reference; it shares the application workflow, it does not bypass it |
| Find implemented work and open gaps | [Status](status.md) |
| Choose future work | [Roadmap](roadmap.md) |
| Inspect historical Markdown-first design | [Design note: finite AI workspaces](superpowers/specs/2026-09-30-markdown-first-ai-workspace-design.md): earlier baseline, not new-work requirements |
| Review existing local Markdown editing | [Earlier open/save/recovery design](superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md), [historical implementation plan](work/active/markdown-note-editing/plan.md), [qualification](work/active/markdown-note-editing/evidence.md): implemented and merged via PR #13; native acceptance pending; new simple Save follows the current roadmap |
| Review current Save/recovery and legacy removal | [Stage 1 results](work/completed/simple-save/plan.md), [Stage 2 results](work/completed/legacy-removal/plan.md): shared manual Save, exclusive Copy, recovery/interruption checks and six-crate production paths; owner native acceptance pending |
| Review existing shell and historical UI ideas | [Workspace shell design](superpowers/specs/2026-10-01-workspace-shell-design.md), [historical UI feature backlog](ui/feature-backlog.md), [shell decision](architecture/decisions/2026-10-01-workspace-shell.md); future slices follow the current roadmap |
| Understand code ownership | [Architecture overview](architecture/overview.md), relevant crate README |
| Review historical Rig/simple-notes work | [Rig-first specification](superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md), [plan pack](work/active/rig-first-reset/plan.md), [evidence](work/active/rig-first-reset/evidence.md), [simple-notes roadmap](work/active/simple-rig-notes/plan.md): historical records, not execution instructions |
| Find the original reset handoff or paused chat-polish ideas | [Historical reset handoff](architecture/brn-rig-first-architecture-reset.md), [paused brainstorm](work/active/ui-slice-2-chat-polish/paused-brainstorm.md); neither authorizes new implementation |
| Change storage, revisions, comments or retrieval | [Invariants](architecture/invariants.md), [completed task records](work/completed/README.md) |
| Change provider behavior | [Rig AI contract](../crates/brn-ai/README.md), [shared workflow](../crates/brn-workflow/README.md). The [standalone sidecar trial](../experiments/codex-app-server/PACKAGING.md) is historical evidence, not a product gate. |
| Qualify actual provider capabilities | [Stage 3 qualification](work/completed/provider-capabilities/plan.md): explicit models, effort, native web/citations and images; bounded live results and explicit qualification gaps |
| Choose and report checks | [Verification](development/verification.md) |
| Start or hand off a task | [Workflow](development/workflow.md), [active work](work/active/README.md) |
| Understand an architectural choice | [Decision index](architecture/decisions/README.md), [dependency record](architecture/dependencies.md) |
| Reproduce an experiment | Its README under [completed work references](work/completed/README.md) |

## Document ownership

- Product vision owns requirements, with resolved owner clarifications captured in the frozen invariants.
- `development/workflow.md` owns the normal development method; `development/verification.md` owns checks. Skills support those rules.
- `status.md` is the current progress summary, scoped to a stated commit and date.
- `roadmap.md` defines future outcomes and prerequisites, not implementation authorization.
- Architecture overview/invariants distinguish the frozen target from existing compatibility contracts. Dated audits/decisions retain rationale and supersession history.
- Crate READMEs describe local responsibilities, interfaces and tests; Rust source remains the API definition.
- Active plans define scope and acceptance. Evidence records observations, commands and limitations.
- Completed records preserve historical results. Their old branch names, machine paths, model assignments and permissions are not current instructions.

When implementation and guidance disagree, inspect the code and evidence, identify whether this is a defect or stale documentation, and reconcile the discrepancy explicitly. Do not silently reinterpret an invariant to fit the implementation.
