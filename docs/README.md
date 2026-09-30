# Documentation index

Start with [AGENTS.md](../AGENTS.md), then [current status](status.md). Read only the references needed for the task.

| Need | Read |
| --- | --- |
| Understand the product or run it | [Root README](../README.md), [setup](development/setup.md) |
| Find implemented work and open gaps | [Status](status.md) |
| Choose future work | [Roadmap](roadmap.md) |
| Review the proposed Markdown-first direction | [Design note: finite AI workspaces](superpowers/specs/2026-09-30-markdown-first-ai-workspace-design.md) (direction approved; detailed defaults under review, not implemented) |
| Understand code ownership | [Architecture overview](architecture/overview.md), relevant crate README |
| Change storage, revisions, comments or retrieval | [Invariants](architecture/invariants.md), [completed task records](work/completed/README.md) |
| Change provider behavior | [Provider README](../crates/brn-provider/README.md), [sidecar packaging boundary](../experiments/codex-app-server/PACKAGING.md) |
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
