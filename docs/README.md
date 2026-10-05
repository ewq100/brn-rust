# Documentation index

Start with [AGENTS.md](../AGENTS.md). The owner froze the reviewed product architecture on 2026-10-03 and accepted the concise [development workflow](development/workflow.md). Use [current status](status.md) when choosing work; read only task-relevant references.

Stages 1–4 are integrated; Stage 5 knowledge and Stage 6 Actions/dashboard
foundations are integrated with native/live/owner qualification open. Stage 7
Inbox is active. Integrated main is `48941d3` (PR60); the unmerged lifecycle
snapshot and architecture-review corrections are distinguished in [status](status.md).

| Need | Read |
| --- | --- |
| Understand the product or run it | [Root README](../README.md), [setup](development/setup.md) |
| Understand desired product behavior | [Product vision](product/BRN_PRODUCT_VISION.md): requirements and future capabilities, not a claim that all are implemented |
| Understand the frozen target | [Architecture overview](architecture/overview.md#frozen-target), [invariants](architecture/invariants.md#frozen-target-guarantees). [Opus audit](audits/BRN_PRODUCT_ARCHITECTURE_AUDIT.md) and [independent review](audits/BRN_PRODUCT_ARCHITECTURE_REVIEW.md) are dated rationale, not execution plans |
| Operate a workspace headlessly as its owner | [brn CLI](../crates/brn/README.md): owner-operated full command authority, `--json` envelopes and the command reference. Approval, Save, completion and removal require applicable owner authorization. Future external agents are read/propose unless explicitly delegated more authority; all consumers share workflow/AppWorker. |
| Find implemented work and open gaps | [Status](status.md) |
| Choose future work | [Roadmap](roadmap.md) |
| Review current Save/recovery and legacy removal | [Stage 1 results](work/completed/simple-save/plan.md), [Stage 2 results](work/completed/legacy-removal/plan.md): shared manual Save, exclusive Copy, recovery/interruption checks and six-crate production paths; owner native acceptance pending |
| Review integrated proposal and Action behavior | [Proposal Core](work/completed/proposal-core/plan.md), [Actions/dashboard](work/completed/actions-dashboard/plan.md): exact review/approval, recovery and operational Actions; qualification gaps remain explicit |
| Review knowledge foundations | [Stage 5 record](work/active/knowledge-foundations/plan.md): identity, provenance, scoped reads and relationships; native/live/owner gaps remain open |
| Continue Stage 7 Inbox | [Inbox plan](work/active/text-email-inbox/plan.md), [supersession](work/active/text-email-inbox/supersession-plan.md), [conflicts](work/active/text-email-inbox/conflicts-plan.md), [original-copy work](work/active/text-email-inbox/original-copy-plan.md); use current status/checkpoint for the correction gate and pending lifecycle integration |
| Understand code ownership | [Architecture overview](architecture/overview.md), relevant crate README |
| Understand durable vault bytes and managed metadata | [Vault format](architecture/vault-format.md), [invariants](architecture/invariants.md) |
| Change storage, revisions, comments or retrieval | [Invariants](architecture/invariants.md), [completed task records](work/completed/README.md) |
| Change provider behavior | [Rig AI contract](../crates/brn-ai/README.md), [shared workflow](../crates/brn-workflow/README.md). The [standalone sidecar trial](../experiments/codex-app-server/PACKAGING.md) is historical evidence, not a product gate. |
| Qualify actual provider capabilities | [Stage 3 qualification](work/completed/provider-capabilities/plan.md): explicit models, effort, native web/citations and images; bounded live results and explicit qualification gaps |
| Choose and report checks | [Verification](development/verification.md) |
| Start or hand off a task | [Resumable checkpoint](development/checkpoint.md), [workflow](development/workflow.md), [tooling](development/tooling.md), [active work](work/active/README.md) |
| Understand an architectural choice | [Decision index](architecture/decisions/README.md), [dependency record](architecture/dependencies.md) |
| Reproduce an experiment | Its README under [completed work references](work/completed/README.md) |

## Historical references

These records retain their existing paths. They are supporting evidence, not
instructions to resume or rename superseded work.

| Need | Read |
| --- | --- |
| Inspect historical Markdown-first design | [Design note: finite AI workspaces](superpowers/specs/2026-09-30-markdown-first-ai-workspace-design.md): earlier baseline |
| Review earlier local Markdown editing | [Earlier open/save/recovery design](superpowers/specs/2026-10-01-open-and-safely-edit-markdown-notes-design.md), [implementation plan](work/active/markdown-note-editing/plan.md), [qualification](work/active/markdown-note-editing/evidence.md): merged via PR13; current Save behavior follows the integrated roadmap work |
| Review existing shell and historical UI ideas | [Workspace shell design](superpowers/specs/2026-10-01-workspace-shell-design.md), [UI feature backlog](ui/feature-backlog.md), [shell decision](architecture/decisions/2026-10-01-workspace-shell.md) |
| Review historical Rig/simple-notes work | [Rig-first specification](superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md), [plan pack](work/active/rig-first-reset/plan.md), [evidence](work/active/rig-first-reset/evidence.md), [simple-notes roadmap](work/active/simple-rig-notes/plan.md) |
| Find the original reset handoff or paused chat-polish ideas | [Reset handoff](architecture/brn-rig-first-architecture-reset.md), [paused brainstorm](work/active/ui-slice-2-chat-polish/paused-brainstorm.md) |

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
