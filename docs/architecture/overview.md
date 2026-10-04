# Architecture overview

The owner froze the reviewed product architecture on 2026-10-03. [Product vision](../product/BRN_PRODUCT_VISION.md) supplies requirements; [invariants](invariants.md) records the frozen guarantees and resolved rules. The dated [audit](../audits/BRN_PRODUCT_ARCHITECTURE_AUDIT.md) and [independent review](../audits/BRN_PRODUCT_ARCHITECTURE_REVIEW.md) explain the decisions, not implementation authorization.

## Frozen target

Keep six production crates: `brn-desktop` and `brn` use `brn-workflow`, which coordinates `brn-store`, `brn-retrieval` and the thin `brn-ai` Rig adapter. Retain WorkStore, current retrieval, AppWorker ownership and the shared CLI workflow. Extend these boundaries for the reviewed product outcomes; no new crate, database, service or agent framework is currently justified.

| Data | Target authority |
| --- | --- |
| Approved knowledge, converted sources, confirmed sent communications, profiles, durable relationships and meaningful history | Vault Markdown and ordinary assets |
| Actions/dependencies, sessions/citation evidence, proposals/comments/unsaved work, Inbox/Needs Review/activity/settings | `brn.sqlite` WorkStore |
| Searchable metadata, passages, embeddings and derived explicit/inferred edges | Disposable `index.sqlite` |
| Unprocessed or incompletely converted intake | Original intake files retained until complete conversion and approval |
| Trash, bounded undo and apply recovery | Ordinary files plus operational receipts |
| Credentials | Existing protected credential directory |

Proposals have one review lifecycle and typed changes enforced by the workflow. Relationships and context use existing records and derived queries; the graph is a view, not another datastore. Source wording, interpretation, approved knowledge and tentative findings stay distinguishable.

Crate boundaries, data ownership and product guarantees stay frozen. Reopen them with the owner only for a changed requirement or a demonstrated blocker the target cannot reasonably handle. Local table, algorithm, tool and UI details can evolve inside those boundaries. [Roadmap](../roadmap.md) owns the reviewed sequence; [status](../status.md) owns actual implementation and qualification.

## Implemented baseline

The sections below describe existing code and compatibility behavior, not proof that the target is implemented. The [2026-09-28 decision](decisions/2026-09-28-architecture-baseline.md) and later simple-notes specifications are historical baselines; their conflicting future rules do not govern new work. Preserve existing safety contracts until a verified replacement or authorized removal. The sample `brn-core` remains in the workspace but is outside the six-crate production target.

## Ownership and dependencies

| Crate | Current responsibility | Workspace dependencies |
| --- | --- | --- |
| [brn](../../crates/brn/README.md) | Agent-facing `brn` CLI over the shared workflow | Workflow |
| [brn-core](../../crates/brn-core/README.md) | In-memory shell, generation correlation and sample worker | None |
| [brn-store](../../crates/brn-store/README.md) | WorkStore operations/editor recovery/save journals; retained legacy revisions, drafts, comments and note registry | None |
| [brn-retrieval](../../crates/brn-retrieval/README.md) | Derived indexes, search profiles and evidence validation | None |
| [brn-workflow](../../crates/brn-workflow/README.md) | Simple AppWorker, Markdown Save/recovery, current-vault read tools and consented model installation; retained legacy local worker/`brn-flow` | Store, retrieval, AI |
| [brn-ai](../../crates/brn-ai/README.md) | Explicit ChatGPT/Copilot authentication, owned Rig clients and streamed read-only answers | None |
| [brn-desktop](../../crates/brn-desktop/README.md) | GPUI workspace shell (history, document/chat, vault), presentation DTOs/layout and sample headless checks | Core, workflow |

`brn-desktop` sends simple work only through AppWorker; retained legacy local
editing/recovery/history uses `brn-workflow::worker`. The headless `brn-flow`
driver retains that legacy workflow. Workflow coordinates authoritative storage, derived retrieval and provider calls; views do not implement those operations themselves.

The simple `brn-workflow::app::App` is implemented alongside that legacy flow.
The CLI uses owned AppWorker/chat/account/model lanes for simple commands;
desktop uses those same lanes, without App/SQL/model/secret state in views.
Native defaults to BRN-simple and its exact BRN-simple.credentials sibling;
old BRN requires explicit legacy mode or legacy markers. App exclusively
owns WorkStore, optionally binds one current vault and shares one loaded
embedder/search policy between Library and read-only AiTools. Both stores enforce
mode exclusion under the same owner lock before SQLite opens. No UI/CLI direct
AI/retrieval/store dependency is added. Installation requires fresh consent;
persisted approval never starts network work.
Legacy CLI local editing/history still uses Workspace; `brn ask` cannot submit
legacy AI work. Advisory classification reuses Store's marker/backup rules.
Simple credential location selection and settings reads occur inside the owning
App lane, not through an extra frontend WorkStore.

## Data flow

1. Selected text/Markdown is imported through the workflow into exact stored versions. Retrieval approval is explicit.
2. Eligible stored content builds a derived index generation. Searches return versioned passages and exact evidence.
3. Simple AppWorker refreshes the vault before new Ask and sends current read-only
   tools to the frozen Rig provider/model. WorkStore owns text-only history and
   durable endings. Legacy thread/turn/evidence/usage fields remain readable,
   but every legacy Ask returns typed LegacyAiRetired without lookup or mutation.
4. Draft working copies are saved separately from immutable checkpoints and retained AI candidates.
5. Comments retain original immutable provenance and expose conservative mappings and independent lifecycle status.
6. Simple Markdown editing and retained legacy notes explicitly save through the shared workflow. Automatic editing-buffer recovery commits unfinished work to SQLite, not to Markdown.

## Simple Markdown Save and recovery

`brn-workflow::editor` coordinates WorkStore's exact editing baselines, buffers,
save/copy intents and receipts through AppWorker. Fresh file observations remain
separate from recovery text. Its private macOS adapter is shared with the retained
legacy path: descriptor-relative containment, vault ownership, Foundation
coordination, attribute-preserving atomic exchange/exclusive installation and
required durability remain enforced. Missing originals are not recreated.

Save binds exact submitted generations and parent/file identity; acknowledgements
preserve later typing. Save Copy uses an independent unused destination.
Reconciliation classifies interruptions from identity proof without repeating
writes. Unresolved saves fence current search/AI tools; uncertain original writes
block further original Save. Confirmed reload binds reviewed disk state and
explicit discard. WorkStore retains one recent Applied recovery pair plus compact
settled receipts after proven artifact retirement; unresolved work and unexpected
artifacts remain protected. Native guarded navigation/close/Quit waits for
acknowledged buffer recovery; Dock/system termination can lose unacknowledged
typing. Implementation and qualification are distinguished in [status](../status.md).

## Managed-note authority

The remaining managed-note contracts describe the retained legacy compatibility
path until Stage 2 removal. For enrolled notes, the live registered Markdown file is the authority for
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
no inherited approval. The same predicate guards legacy approval, index publication and
all legacy search profiles. Current document surfaces
pair valid content with explicit exclusion states; associated old imports are
shadowed, not deleted. Unrelated legacy imports retain snapshot semantics.
Changed corpus/epochs can still invalidate the whole index.

See the [implemented qualification and pending native acceptance](../work/active/markdown-note-editing/evidence.md).
Coordination covers participating writers only; arbitrary late races can be
detected after installation. File durability uses `F_FULLFSYNC`, directories
use explicit `libc::fsync`; process-kill tests do not establish power-loss
durability or other-volume support. Native UI acceptance remains pending.
Window close, application Quit/Cmd-Q and note switching guard recovery.
Guarded native close/Quit asynchronously cancel and join local work before
closing; finalization errors retain explicitly unsaved partials and block close.
Pinned
GPUI cannot veto Dock/system termination, and no final-hook flush was added.
Unacknowledged typing on that route can be lost.

Comment-batch generation and publication were legacy future interfaces. They are superseded by the frozen whole-proposal target and are not capabilities established by this description of the current flow.

## Build boundaries

Default desktop features are empty. `native-ui` enables GPUI and the integrated workflow; `native-retrieval` additionally enables workflow native retrieval. Workflow's `native-retrieval` enables retrieval's `native` feature. Keyword paths do not require model assets. Experiments are standalone manifests with separate lockfiles, outside the root workspace.

The desktop's layout.json in the data directory is presentation state only; deleting it restores default layout without affecting authoritative data.

Read [invariants](invariants.md) before changing a boundary, and [dependencies](dependencies.md) for dated dependency observations and distribution limits.

## Historical reset and remaining simple-app work

The [Rig-first reset](../superpowers/specs/2026-10-01-rig-first-architecture-reset-design.md),
old provider plans and [simple notes roadmap](../work/active/simple-rig-notes/plan.md)
are historical implementation/design evidence. New work follows the frozen target
and current [roadmap](../roadmap.md).
Rig subscription chat, protected file credentials and saved-vault retrieval are
implemented and merged through PR #14; production App Server is retired.
Simple manual Save/recovery is implemented and automated checks have passed;
native acceptance remains pending in [status](../status.md).
Proposal/approval tools and legacy removal follow the current roadmap;
the old Steps 5/6 order is superseded.
`brn-core` still serves sample/headless shell behavior.
Build/state tests do not establish graphical usability, live-provider acceptance
or release readiness.
