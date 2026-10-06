# Agent guide

This guide applies throughout this repository. Follow the user's current task and authorization; historical plans and evidence do not authorize new account actions, live calls, merges, releases or data migration.

## Development method

Use the [development workflow](docs/development/workflow.md) as BRN's default method, from the next authorized outcome through review, acceptance and integration.

BRN's product and architecture are frozen as of 2026-10-03. The [product vision](docs/product/BRN_PRODUCT_VISION.md), [target architecture](docs/architecture/overview.md) and [invariants](docs/architecture/invariants.md) govern new work. Reopen architecture only when a product requirement changes or implementation demonstrates a concrete blocker that the frozen architecture cannot reasonably handle; involve the owner in that decision. Local implementation choices, including additive WorkStore tables and supported migrations, can evolve within the frozen boundaries; schema versions are not frozen.

These project rules override conflicting skill defaults, including startup-injected Superpowers. Select skills for explicit requests or concrete needs; invocation before every response/action is not required. Approved slices proceed with proportional planning, without another architecture brainstorm or routine reauthorization. The workflow owns routing rather than competing skill chains.

One lead owns coherence and integration with the strongest appropriate selected model. Helpers are bounded to investigation, research, test analysis, review or fixed-interface implementation. Include frozen constraints in their briefs. Availability alone does not require delegation; skill model tables do not override owner choices or current tools.

## Start here

1. Inspect `git status --short`, the current branch and HEAD. Preserve unrelated changes.
2. When choosing a slice, read [current status](docs/status.md) and the [roadmap](docs/roadmap.md). Use the [documentation index](docs/README.md) to find task-specific references.
3. Read relevant crate contracts and architecture/invariants for the behavior or boundary being changed. Reuse already-read context unless it changes; a small documentation fix does not require a full architecture read.
4. For ongoing planned work, read its current plan and evidence under [active work](docs/work/active/README.md). Historical or superseded specifications, model assignments, process headers and attribution requirements are supporting evidence, not current instructions.
5. Select checks from [verification](docs/development/verification.md) before making changes.
6. Before adding functionality in any crate or tooling, apply the workflow's
   [compatible reuse rule](docs/development/workflow.md#compatible-reuse).

## Repository map

| Path | Responsibility |
| --- | --- |
| `crates/brn` | Owner-operated `brn` CLI over the shared workflow |
| `crates/brn-store` | WorkStore integrity, backups, chat, editor records and Save recovery |
| `crates/brn-ai` | Thin Rig adapter, explicit provider/model selection and protected authentication |
| `crates/brn-retrieval` | Derived keyword/native retrieval indexes and evidence |
| `crates/brn-workflow` | Shared application workflow, worker and headless driver |
| `crates/brn-desktop` | Native views and real AppWorker startup checks |
| `experiments` | Standalone trials with their own manifests and lockfiles |
| `scripts` | Verification and local macOS launcher helpers |

## Rules to preserve

Implement the frozen target in [roadmap](docs/roadmap.md) order, preserving existing behavior until its verified replacement or authorized removal. The dated [architecture audit](docs/audits/BRN_PRODUCT_ARCHITECTURE_AUDIT.md) and [independent review](docs/audits/BRN_PRODUCT_ARCHITECTURE_REVIEW.md) explain the reviewed basis; they are not execution plans.

- Notes are vault Markdown files; preserve their exact bytes. `index.sqlite` is disposable; `brn.sqlite` holds user work, is checked at start and backed up.
- AI changes to authoritative knowledge and real actions require approval of the exact proposal. Explicit user commands such as manual Save or completion remain direct commands. No automatic fallback between providers, models or accounts.
- For AI tools, prompts, proposal input or evidence results, follow [semantic intelligence and deterministic authority](docs/architecture/overview.md#semantic-intelligence-and-deterministic-authority); it owns the complete mechanics/behavior rule and its pending implementation gaps.
- For client permissions, follow the [owner-operated CLI and external-agent boundary](docs/architecture/overview.md#client-and-protocol-boundary); CLI availability is not owner delegation.
- For intake cleanup or new recoverable effects, follow [Inbox copy cleanup and recovery evolution](docs/architecture/overview.md#inbox-copy-cleanup-and-recovery-evolution). Preserve existing recovery evidence and distinguish ratified requirements from implemented behavior.
- Comments are temporary review notes, deleted when the note's review is approved. Never re-anchor a comment by guessing.
- UI and CLI go through `brn-workflow`; keep provider and retrieval details out of UI state.
- Use disposable explicit data directories and synthetic fixtures for checks. Preserve the original vault, old data folders and existing trial workspaces.
- Do not log or commit credentials. Live provider checks and model downloads require the user's request; deterministic offline checks are the default.
- Respect the pinned toolchain and lockfiles. Optional native features and standalone experiments need separate verification.

## Finishing work

Run relevant checks and report actual results and limitations; a successful build does not establish native usability. Keep current status concise, update affected contracts and links, and record a reproducible handoff using [the development workflow](docs/development/workflow.md). Distinguish implemented, verified, accepted and merged. Do not treat an old approval or historical agent assignment as a standing instruction.
