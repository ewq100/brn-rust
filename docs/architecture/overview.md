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

The six target crates are the production workspace. The retired Store/Workspace,
legacy worker/CLI/native paths, `brn-core` sample shell and `brn-flow` driver are
removed. Historical decisions and experiments remain evidence. Old or mixed
database/backup markers are refused before SQLite opens; no old data is migrated.
[Status](../status.md) records actual verification and acceptance.

| Crate | Current responsibility |
| --- | --- |
| [brn](../../crates/brn/README.md) | Agent-facing CLI over AppWorker |
| [brn-desktop](../../crates/brn-desktop/README.md) | Native presentation, exact text buffer, layout and AppWorker commands |
| [brn-workflow](../../crates/brn-workflow/README.md) | Application/chat/account/model lanes, vault, Save/recovery and read-only AI tools |
| [brn-store](../../crates/brn-store/README.md) | WorkStore integrity, backups, chat, proposal review, exact editor recovery and Save journals |
| [brn-retrieval](../../crates/brn-retrieval/README.md) | Disposable current-note FTS5/embedding index and evidence |
| [brn-ai](../../crates/brn-ai/README.md) | Explicit account/model selection, protected authentication and Rig streaming |

App owns WorkStore, optionally binds one current vault and shares one loaded
embedder/search policy between Library and AiTools. UI and CLI keep SQL, provider
and retrieval implementation out of presentation state. AppWorker owns admission,
cancellation and joined work; credential selection/settings reads happen inside
its lane. Native defaults to BRN-simple and its protected credential sibling.

## Simple Markdown Save and recovery

`brn-workflow::editor` coordinates exact WorkStore baselines, buffers, intents
and receipts. Fresh file observations remain separate from recovery text. The
private macOS file adapter validates descriptor-relative containment, root/parent
identity, ownership and attributes; it requires Foundation coordination,
attribute-preserving atomic exchange/exclusive installation and durability.
Missing originals are not recreated. Copies use independent unused destinations.

Save binds submitted generations. Acknowledgements preserve later typing.
Reconciliation uses prepared/installed/displaced identity proof without repeating
writes. Unresolved saves fence search/AI and further original Save. Reload binds
reviewed disk state and explicit discard. WorkStore retains the latest Applied
recovery pair and compact settled receipts after proven artifact retirement;
unresolved work and unexpected artifacts stay protected. Guarded navigation and
quit wait for acknowledged recovery. System quit drains admitted work but cannot
promise unadmitted typing.

## Chat, search and models

New Ask refreshes the vault, freezes explicit provider/model selection and uses
current read-only tools. Streaming is provisional; durable terminal pairs and
visibly unsaved partial failures stay distinct. Bound UUID replay never submits
another provider request. Chat/account/read leases drain before authority releases.

Fresh Ask also captures an explicit low/medium/high reasoning effort, persisted
independently in settings and paired V7 history. The thin Rig adapter sends that
choice on the selected route. Setting changes affect new requests; historical
unknown effort remains unknown and replay never initiates another provider call.

Current retrieval is derived from supported saved files. Dirty recovery is not
current source evidence. Results validate fresh bytes; results spanning Save or
unresolved work are rejected. Default builds explicitly use keyword-only search;
optional native features load FastEmbed/ONNX and require fresh consent for model
installation. Startup and saved consent never initiate downloads or login.

## Typed review foundation

WorkStore V4 and AppWorker now persist typed Markdown drafts, exact before/source
bindings, full edits, temporary comments and rejection. One review version guards
late Rewrite results; uncertain anchors retain their old range without guessing.
Draft/edit/comment operations do not apply knowledge. Exact reviewed approval
now applies through AppWorker and the CLI, preserving whole-proposal proof and
later editor work. Shared activity, explicit Undo/Trash and proof-bound human
Finish/Restore repair use the same typed application/recovery boundary. Owned AI
Rewrite uses the existing chat lane and one narrow V6 operational job, committing
validated review text and outcome atomically against its captured stamp/hash.
Jobs retain safe metadata only; restart interrupts without retry. Native full
review/edit/comments and Rewrite controls use persistent text widgets and guarded
AppWorker acknowledgements. Native exact individual/group approval captures full
records and binds every outcome; activity/reconciliation use shared receipts.
Native Undo/repair and initial proposal creation remain Stage 4 work before later
domains extend typed changes.

WorkStore V5 adds exact approval snapshots, all-member prepared proofs and
whole-proposal receipts. Pending/Uncertain journals fence current reads and
conflicting Save/reload while keeping review and unfinished editor work readable.
Storage does not install files. Workflow stages all members, persists prepared
proofs in SQLite and bounded ordinary recovery receipts, then uses coordinated
exchange/exclusive installation and required durability. Whole completion is
persisted before SQLite finalization. Every startup imports retained records
before binding current evidence, including healthy older/fresh databases;
completed historical replay preserves subsequent user file edits. Partial or
incompatible proof stays fenced. Ordinary receipt/comment cleanup never removes
unexpected artifacts. These receipts implement the frozen ordinary-file recovery
boundary rather than adding another datastore.

## Build boundaries and remaining work

Default workspace checks exclude optional native UI/retrieval. Standalone
experiments retain their own manifests/lockfiles and are outside production.
Build/state tests do not establish GUI usability, real inference, provider
capability or release readiness. Proposal Core and the remaining product stages
follow the [roadmap](../roadmap.md); no generic publication or workflow framework
is retained from earlier designs.
