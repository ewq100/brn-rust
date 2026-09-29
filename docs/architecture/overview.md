# Architecture overview

This describes the implemented workspace at the baseline in [status](../status.md). The [approved architecture decision](decisions/2026-09-28-architecture-baseline.md) records intent and rationale; the original proposal's `core` role is currently split between the sample `brn-core` and integrated `brn-workflow`.

## Ownership and dependencies

| Crate | Current responsibility | Workspace dependencies |
| --- | --- | --- |
| [brn](../../crates/brn/README.md) | Agent-facing `brn` CLI over the shared workflow | Workflow |
| [brn-core](../../crates/brn-core/README.md) | In-memory shell, generation correlation and sample worker | None |
| [brn-store](../../crates/brn-store/README.md) | SQLite records, migrations, exact revisions, drafts, comments and recovery | None |
| [brn-provider](../../crates/brn-provider/README.md) | App Server client and owned process lifecycle | None |
| [brn-retrieval](../../crates/brn-retrieval/README.md) | Derived indexes, search profiles and evidence validation | None |
| [brn-workflow](../../crates/brn-workflow/README.md) | Integrated application operations, worker commands/events and `brn-flow` CLI | Store, provider, retrieval |
| [brn-desktop](../../crates/brn-desktop/README.md) | GPUI views, interaction state and sample headless checks | Core; workflow with native UI enabled |

`brn-desktop` sends integrated application work through `brn-workflow::worker`. The headless `brn-flow` driver uses the same workflow. Workflow coordinates authoritative storage, derived retrieval and provider calls; views do not implement those operations themselves.

## Data flow

1. Selected text/Markdown is imported through the workflow into exact stored versions. Retrieval approval is explicit.
2. Eligible stored content builds a derived index generation. Searches return versioned passages and exact evidence.
3. Grounded answers use validated evidence; local session records associate with provider-owned threads.
4. Draft working copies are saved separately from immutable checkpoints and retained AI candidates.
5. Comments retain original immutable provenance and expose conservative mappings and independent lifecycle status.

Comment-batch generation and publication are future interfaces, not capabilities established by this diagram of the current flow.

## Build boundaries

Default desktop features are empty. `native-ui` enables GPUI and the integrated workflow; `native-retrieval` additionally enables workflow native retrieval. Workflow's `native-retrieval` enables retrieval's `native` feature. Keyword paths do not require model assets. Experiments are standalone manifests with separate lockfiles, outside the root workspace.

Read [invariants](invariants.md) before changing a boundary, and [dependencies](dependencies.md) for dated dependency observations and distribution limits.
