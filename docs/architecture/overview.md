# Architecture overview

This describes the implemented workspace at the baseline in [status](../status.md). The [approved architecture decision](decisions/2026-09-28-architecture-baseline.md) records intent and rationale; the original proposal's `core` role is currently split between the sample `brn-core` and integrated `brn-workflow`.

## Ownership and dependencies

| Crate | Current responsibility | Workspace dependencies |
| --- | --- | --- |
| [brn](../../crates/brn/README.md) | Agent-facing `brn` CLI over the shared workflow | Workflow |
| [brn-core](../../crates/brn-core/README.md) | In-memory shell, generation correlation and sample worker | None |
| [brn-store](../../crates/brn-store/README.md) | SQLite records, migrations, exact revisions, drafts, comments, managed-note registry and unfinished-work recovery | None |
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
6. Managed Markdown notes open and explicitly save through the shared note workflow. Automatic editing-buffer recovery commits unfinished work to SQLite, not to Markdown.

## Managed-note authority

For enrolled notes, the live registered Markdown file is the authority for
current saved content. SQLite owns vault/note/path identity, search permission,
editing baselines and buffers, save/copy intents, recovery pairs and compact
operation results. Immutable imported originals, revisions, comments and
conversation provenance retain their original identities and bytes. Search
snapshots and indexes are derived; stored recovery or historical bytes never
substitute for a fresh saved-file observation.

`brn-workflow::notes` owns observation, generation checks, explicit Save,
comparison, confirmed reload/relink, exclusive save-copy and restart
reconciliation. Its private macOS adapter owns bounded descriptor-relative
reads, directory-descriptor vault ownership, Foundation coordination/presenter
notifications, atomic exchange/exclusive installation and required durability.
The desktop uses the owned workflow worker; the CLI uses the same operations.
Neither interface independently writes Markdown or executes SQL.

One workspace binds one vault. Opening SQLite and inspecting recovery/history
does not require that vault to be available; filesystem operations lazily
validate/acquire its registered identity. Missing/replaced or separately owned
roots expose unavailable/owned-elsewhere state without claiming stored bytes
are current. Aliases and overlapping roots cannot bypass ownership.

The shared eligibility predicate requires a supported existing file at its
reconciled location, no unresolved original-path save, and a fresh exact
identity/content match to its explicitly approved search snapshot. Recovery
buffers and artifacts are never eligible; a copy has independent identity and
no inherited approval. The same predicate guards approval, index publication,
all search profiles and provider handoff/completion. Current document surfaces
pair valid content with explicit exclusion states; associated old imports are
shadowed, not deleted. Unrelated legacy imports retain snapshot semantics.
Changed corpus/epochs can still invalidate the whole index.

See the [implemented qualification and pending native acceptance](../work/active/markdown-note-editing/evidence.md).
Coordination covers participating writers only; arbitrary late races can be
detected after installation. File durability uses `F_FULLFSYNC`, directories
use explicit `libc::fsync`; process-kill tests do not establish power-loss
durability or other-volume support. Native UI acceptance remains pending.
Window close, application Quit/Cmd-Q and note switching guard recovery; pinned
GPUI cannot veto Dock/system termination, and no final-hook flush was added.
Unacknowledged typing on that route can be lost.

Comment-batch generation and publication are future interfaces, not capabilities established by this diagram of the current flow.

## Build boundaries

Default desktop features are empty. `native-ui` enables GPUI and the integrated workflow; `native-retrieval` additionally enables workflow native retrieval. Workflow's `native-retrieval` enables retrieval's `native` feature. Keyword paths do not require model assets. Experiments are standalone manifests with separate lockfiles, outside the root workspace.

Read [invariants](invariants.md) before changing a boundary, and [dependencies](dependencies.md) for dated dependency observations and distribution limits.
