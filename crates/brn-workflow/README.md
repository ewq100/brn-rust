# brn-workflow

> Requirements/qualification context (2026-10-07): this README describes implemented behavior, not mandatory limits or acceptance of the proposed replacement. The [owner amendment](../../docs/product/BRN_PRODUCT_VISION.md#owner-amendment--2026-10-07), [reassessment](../../docs/audits/BRN_ARCHITECTURE_REASSESSMENT_2026-10-07.md) and [proposed plan](../../docs/work/active/architecture-reassessment/plan.md) reopen mechanisms. Email enum/literal text is not real EML ingestion; broader conversion, AI draft freedom/budgets and practical reviewability remain gaps. No production behavior changed in this documentation task.

Shared client-facing application boundary for desktop, CLI and future protocol
adapters: manual Markdown Save/recovery, knowledge operations and Rig chat/read/
search through AppWorker. Legacy production Store/Workspace/worker and brn-flow
paths are removed.

The [architecture client boundary](../../docs/architecture/overview.md#client-and-protocol-boundary)
permits a future thin read-only stdio MCP adapter. It maps capabilities to this
crate's commands/events; it does not open vault/SQLite files or implement ranking,
knowledge scopes, parsing, relationships, provider or proposal rules. External
search/read/list default to Current; other scopes are explicit. Retrieval remains
replaceable inside the core. No MCP implementation, daemon or remote service is
part of this amendment.

Search results expose workflow-owned `NoteHit` evidence (path, whole-note hash,
byte range, exact quote and relative score). SQLite passage IDs stay inside
retrieval for ranking/fusion and are not client identifiers. Domain scope and
relationship DTOs retain their current meaning; adapters must not depend on
retrieval storage details.

## Interfaces and source

[Application](src/app.rs), [commands/events](src/app_worker.rs), [chat lane](src/chat_worker.rs), [editor](src/editor.rs), [Inbox boundary](src/inbox.rs), [proposal review](src/proposals.rs), [proposal application](src/proposal_apply.rs), [durable provenance](src/knowledge/provenance.rs), [activity](src/activity.rs), [file adapter](src/files/mod.rs).

`BackupStatus` and `CheckpointBackup` return the typed [internal checkpoint status](src/backups.rs)
through AppWorker. Manual requests require a nonzero UUID; automatic status events
use nil and never acknowledge an unrelated request. The owned lane checks due
work between commands and on an idle wake, with at most one automatic attempt
per minute and no catch-up loop. Only changed internal state is copied. After
admitted mutations and attached chat work settle at shutdown, a final checkpoint
captures later writes. Copy failures remain separate status/warnings, preserving
the original operation result and owner buffers. There is no provider request,
new scheduler, schema or automatic rollback. Copies retain the existing five-newest
policy; vault files and external retained artifacts remain separate recovery inputs.

## Inbox evidence boundary

`InboxRetainedExtractions(capture_id)` discovers checked immutable extraction
versions for the exact retained Inbox catalog item. Equal-byte imports remain
separate. Results follow `(batch_id, index, snapshot_id)` identity order, with no
current/latest preference. The operation performs no conversion, original-file
access, provider request or approval. Catalog absence, corruption or bounded-scan
refusal is explicit; standalone `InboxExtraction(snapshot_id)` history remains
independent of the catalog/queue. Discovery does not refresh approval proofs.


`ProcessInbox`, `InboxProcessing`, `InboxCandidate` and `CancelInboxProcessing`
extend the same headless boundary. One joined AppWorker lane advances one member
at a time between commands. Admission binds 1–8 complete retained Inbox snapshots;
at most 16 members can be pending. WorkStore V13 keeps exact requests, states and
conversion receipts. Restart interrupts unfinished members; replay never silently
restarts work. New UUIDs explicitly retry retained originals.

Markdown previews preserve exact bytes; deliberate Text/Teams copies use safe
literal Markdown fences around the exact body. Email uses maintained MIME extraction
as described below. Oversized previews are flagged without truncation. Literal
preview reads recheck originals; maintained previews read immutable retained
snapshots even after original loss. Fresh effects require separately qualified
original/evidence proofs. Imported frontmatter remains pending text rather than managed metadata.
`InboxConversionPreview::validate_receipt` checks the complete batch/original/
format/length/digest at this boundary. Clients need no hashing or conversion logic;
a complete operational preview remains readable even if a later Source wrapper
would exceed its separate byte bound.
These previews require semantic review. `PrepareInboxSource` prepares a bound
whole Source draft for the existing exact approval/recovery boundary. Opt-in
knowledge, Action and supersession review are described below.
Conflict capture and explicitly confirmed text-copy Remove/Restore are described
below. The bounded DOCX inline PNG/interpretation profile is described below;
the maintained EML/DOCX profile below supports plural attachments and images.
No original deletion, provider call or authoritative write occurs here.

`CaptureInbox`, `InboxItem` and `InboxItems` are headless workflow commands for
explicit bounded text, Markdown, email and Teams copies. AppWorker admits capture
as a critical mutation and coordinates the private ordinary-copy namespace,
immutable WorkStore receipt, exact replay, availability reporting and startup
recovery. The workflow validates labels, UUIDs, copy identity and complete UTF-8
text; clients never supply a destination path or open SQLite/files themselves.

Original bytes are retained outside the vault and retrieval index. A typed mirror
is durably published before exclusive UUID.txt installation, then the installed
file, mirror and directory are rechecked and synchronized before acknowledgement.
Unknown, partial, changed, aliased or replaced artifacts stay in place and appear
as explicit Inbox issues. A known full receipt can reconcile after restart or
database backup restoration; equal bytes at another inode are insufficient.
Missing originals are reported without claiming fresh source evidence. Capture
does not convert content, call a provider, create knowledge/proposals, or delete
copies. Future protocol adapters should call these workflow commands rather than
the file or Store modules.

`CaptureBinaryInbox` uses the same capture family and `InboxCaptured` reply for
explicit opaque originals up to 16 MiB, including empty and invalid UTF-8 bytes.
`CaptureBinaryInboxRequest` binds UUID, labels and complete bytes; text capture
rejects Binary. UUID.bin is derived from the typed receipt, while existing text
kinds retain UUID.txt and their 1 MiB limit. Capture refuses either occupied suffix;
recovery never substitutes one suffix for the other. Catalog schema V16 and the
metadata-only format1 capture mirror remain unchanged.

`InboxOriginal::AvailableBinary` reports only complete length/hash after a fresh
stable private-file identity/byte observation. `InboxRead::validate_receipt`
checks client DTO consistency; it does not establish fresh filesystem authority.
Binary processing selects EML by its captured `.eml` name and otherwise DOCX;
the Email text kind also invokes the maintained MIME helper. The owned helper is
restricted before stdin, uses empty environment and bounded pipes, and is joined
on cancellation, timeout and failure. Complete output is validated before an
immutable hashed extraction snapshot and private recovery mirror are published.
Candidates and historical review read that snapshot without reconversion. Fresh
Source approval still checks the exact original file, snapshot, body and every
asset. Unsupported attachments remain retained and visibly unprocessed.

`InboxIntakeBinding` binds retained investigation to the exact pending or
approved Draft stamp, snapshot digest, note identity and text. For an Applied
Source, `intake_analysis_binding` requires exactly one successful journal matching
the current Applied review stamp and full draft; it retains that journal's
original approved Draft stamp rather than substituting the later Applied stamp.
No latest-receipt selection or new Source proposal occurs. Rejected/applying/
uncertain states and missing/ambiguous receipt matches refuse. `AnalyzeInboxActions`
accepts either this binding or the historical saved Source profile. Private
knowledge citations bind exact source-node text ranges; Actions retain the same
prerequisite. No SourceVersion is fabricated for unsaved evidence. Related
knowledge, History and Actions use existing proposals, comment/Rewrite and
application journals. Group approval validates selected stamps and applies the
Source prerequisite before dependent members. Individual dependent approval
requires its exact Applied Source receipt for the current review version,
current saved bytes/assets and fresh unique Source identity. The internal proof
scan remains usable during admitted application while public Current tools stay
fenced. Fresh creation, individual approval and group prerequisite validation
open the checked file adapter after restart even for an Action with no file
changes, before an editor or investigation has been opened. Saved turns replay without fresh inference. The model
input names pending versus Applied Source authority explicitly; neither implies
semantic completeness.

Old DOCX drafts cannot approve or Finish through reconversion. Original bytes and
saved records remain available; exact-proof Restore can undo partial effects,
while absent/changed proof blocks recovery and preserves artifacts. Renewed
extraction and review create new authority. Binary copy cleanup remains refused.

The historical saved-Source profile of `AnalyzeInboxActions` accepts one complete managed Inbox Source, an operation
UUID, optional conversation and explicit provider/model/effort. Full Source bytes
including metadata must fit 50,000 bytes; no truncation occurs. New admission
checks the fresh full file proof and unique UUID. WorkStore V14 retains the exact
capture and generated question; the existing owned Ask turn holds execution
status, cancellation and restart interruption. No second queue/lifecycle exists.
Exact replay precedes fresh Source/choice/account checks, never repeats a retained
turn, and treats presentation generation as transient. An admitted record with a retained budget and no
turn permits explicit retry after fresh validation; historical missing-budget
reservations require a fresh run UUID; ordinary Ask/Rewrite cannot
adopt its UUID. `InboxActionAnalysis` returns capture, optional turn and grouped
review records, always with semantic review still required.

The bound Action capability requires exactly one Action change per proposal,
the unchanged captured Source proof. Workflow injects that proof as the first
source and its note UUID exactly once in Action sources; AI source_paths names
additional evidence and must not repeat the selected path. It assigns the
analysis UUID as group and permits at most20 separate proposals; exact creation
replay preserves later review edits even after Source loss. Existing edit/comment/
Rewrite/reject and exact approval apply. These drafts cause no real Action effect
before approval. History is explicit Source evidence, while context tools default
to Current. `ProposalEvidenceSource` captures full readonly evidence including
archive; the existing current-only `ProposalSource` and destination rules remain.
No analysis outcome authorizes original deletion or establishes complete ingestion.

## Approved Action read tools

Ask and owned Rewrite use fixed `read_action(id)` /
`list_actions(state?, limit?, cursor?)` callbacks through AppWorker's existing
application lane. AI owns protocol parsing/bounds only. Workflow checks domain
UUID/state/opaque ActionCursor, then calls `App::action/actions` freshly, preserving
full immutable origins/current fields and the current-evidence fence. List default
is all labeled operational states, limit1–20/default20, cursor≤256bytes; complete
encoded JSON≤1MiB. AI read_action adds a typed checked_ref (id/version/full-record
SHA256), while owner Action commands preserve their existing output. Oversized
pages refuse whole; callers may reduce the limit.

Each callback has a private reply channel and never reads frontend events or SQL.
The handle shares AppWorker's admission mutex: enqueue under the fence, release it
before waiting, reject after stopping. Every admitted read settles/refuses before
Shutdown. Fatal application-loop errors close admission and refuse queued replies
before joining chat, preventing retained-read deadlock; discovery settlement stays
joined. Existing DrainedTools retains blocking calls through Stop/disconnect/
model changes/quit. The wrapper retains the existing note Arc and all six scoped/
unscoped note methods. Rig uses spawn_blocking, two concurrent tools, the captured investigation
round limit (default8) and zero invalid-tool retries. AI remains vault-bound and explicitly selected.
No real Action mutation, Complete, approval, Save, account or generic dispatch
tool is exposed. Ask receives the separate review capability below.

## Ask Action review capability

Ask uses separate fixed `ProposalTools` through the same owner lane/fence.
Workflow captures the admitted turn and canonical selection/effort, infers its
session and validates complete Create/Replace data, immutable origins/full CAS,
references and full saved source proofs (including explicitly supplied history).
All14 semantic after-fields are required, including explicit nullable values.
AI supplies no proposal/Create UUID or full Replace before-record. Rust derives
proposal/member version8 UUIDs from exact ordered input scoped to the owned turn.
Replace uses the id/version/full-record SHA256 reference returned by read_action;
Workflow loads the entire checked baseline. Relationships use existing UUIDs or
1-based member indices resolved before the established graph validators.
Original replay uses retained full baselines and ordered proofs before fresh
observation, preserving newer review/Source loss/Action advancement through the
original creation-hash check. Changed input or another turn is a distinct draft
that needs fresh validation. No model-supplied session or approval authority exists.

The complete receipt is bounded before durable creation and contains only
stamp/state/session/member UUIDs/source bindings. Admitted mutations drain after
Stop/Quit; late callbacks refuse under the shared cancellation fence. Fatal drains
refuse queued private replies before joining retained read/proposal leases. Either
capability retains that same lease, so terminal settlement/restart follows the
last callback. Rewrite cannot create unrelated proposals. Existing exact approval
is the only producer of real Actions; this adds no schema/Store writer or framework.

## Simple app owner and read tools

[`App`](src/app.rs) opens one `WorkStore` in a new explicit data folder and
exposes its `OpenReport`, including the restored backup. History and settings
work without a vault. A missing initial vault stays unbound; an unavailable
previously bound vault returns `VaultUnavailable` for reads. The first
`bind_vault` persists the canonical root only after Library/reader initialization
succeeds. It refreshes saved knowledge when no durable change is unresolved;
otherwise current evidence remains fenced for reconciliation. A different root needs a different data folder. Vault
and data folders cannot overlap.

Credentials must be absolute, outside Git repositories, the data folder and
the vault. `default_credentials_dir(data_dir)` supplies a canonical sibling
`<data-name>.credentials` (desktop `BRN-simple.credentials`); `Auth::open`
checks only that safe folder, never provider caches. There is no default
provider/model. `AppConfig.credentials_dir: Option<PathBuf>` uses an explicit
location when supplied, otherwise the owner's saved `ai.credentials_dir`, then
the safe sibling. The non-secret absolute location is persisted only by App's
owning lane; frontends never open an extra WorkStore to read settings.
Selection is format-validated and saved atomically in one `ai.selection`
setting. Reading that saved choice remains possible if discovery later removes
its model; no refresh selects a replacement. Explicit discovery results go
through `record_models`; new Copilot selections/turns check recorded membership
without network or credential-cache access. ChatGPT identifiers remain explicit;
the thin provider adapter returns current subscription picker options and reports
upstream refusal rather than substituting another model.

`effort`/`select_effort` expose an explicit low/medium/high choice in `ai.effort`
without provider access. Fresh Ask requires a captured effort before admission;
setting changes cannot alter the owned turn. Paired history records that choice.
Existing unknown-effort turns remain readable and replayable before current
settings, vault or credential checks, without another provider request.

`notes` and `search` refresh before returning CLI reads; `note` reads exact
current knowledge bytes. Their `*_scoped` variants and AppWorker `ScopedNotes`,
`ScopedNote` and `ScopedSearch` explicitly select Current, Source, History or All.
Current excludes sources and historical notes; Source includes original sources
in any state; History includes historical knowledge and sources. All includes
every eligible classified note. Call `refresh` on explicit Refresh, focus and application
writes. `embed_pending(batch)` provides bounded-batch progress for the owned
application lane. [`AiTools`](src/ai_tools.rs) is `Send + Sync`, with a mutexed,
retrieval-owned read-only reader and a clone of Library's one
[`SharedEmbedder`](src/library.rs). Both use `search_index`: absent models flag
**every mode** keyword-only; present models keep Keyword non-downgraded, embed
Semantic once and fuse Hybrid's top 50 keyword/semantic passages. Model
identity/dimension mismatches are errors, never fallback.

Indexed note titles come from saved bytes and never change them. Refresh counts
the first 50 physical lines after an optional BOM, header lines included. It
skips a complete leading header through Store's `note_identity::body_start`
(`---` opening, `---` or `...` closing). When that reader refuses a malformed
managed layout, the note stays indexed and the existing exact `---` framing
adapter locates the body, or the whole text is the body. The pinned Markdown
1.0.0 parser then decides block structure, so headings inside fenced or
indented code and HTML blocks never count. The title is the first level-1
heading whose physical line starts exactly `# ` and whose trimmed raw suffix is
nonempty. Inline markup, links, escapes and closing hashes stay literal. Setext,
indented, `#NoSpace`, `#`-tab and lower-level headings never count. A header
that ends after line 50, a parser failure or no eligible heading gives the file
name without `.md`. The parser skips inline constructs and stops at the last
candidate line. Refresh re-derives titles for unchanged bytes and re-indexes a
note whose stored title differs, which drops its cached passage vectors once.
Pinned Markdown 1.0.0 panics on a setext heading, an adjacent `---` break and
another setext heading. Refresh catches that and falls back, but the default
panic hook still writes the message to stderr.

Every tool revalidates visibility and fresh bytes. Search checks the hash,
UTF-8 range and exact quote before exposing any candidate; stale/removed
notes return safe `IndexStale`. Unsafe arguments return `ToolRejected`.
Reads preserve the exact UTF-8 prefix up to 50,000 bytes, including BOM/CRLF.
Lists validate component-only folders and note-path cursors, use exclusive
path-sorted keyset pagination and return at most 200 entries. `"work"` never
matches `"workshop"`. These APIs never write vault files.

## Retained Actions

Initial Action composition uses the existing typed `CreateProposal` request and
exact approval lifecycle, including source-free requests without a vault.
`ProposalSource::validate` checks returned evidence path, byte bound and exact
text/hash/length consistency without filesystem access; admission still observes
current authority and validates Action references in this application. Clients
retain raw input and returned proofs rather than parsing identities or files.

[Actions](src/actions.rs) expose checked operational records through `App::action`
and `App::actions`, and correlated `Action`/`Actions` worker commands/events. Reads
return the full immutable approved origin and exact current replacement baseline.
No vault, model or provider is required. Pending or uncertain Save/proposal/completion work
fences these current reads until reconciliation. Nil IDs, limits outside 1–200 and
invalid cursors are rejected before authority reads; absent IDs return NotFound.
Pages default to all states/25 entries, with optional state filtering and an
exclusive immutable creation-time/UUID cursor. Editing an Action does not reorder
it. Exact Action creation/replacement uses the proposal lifecycle below. Dashboard
controls use this boundary; owned Action Rewrite uses the lifecycle below. Action-bearing Undo remains refused.

[Dashboard](src/dashboard.rs) exposes `App::action_dashboard` and the correlated
`ActionDashboard` command/event. Default Active is Open/Waiting/Blocked; explicit
filters also expose each state, overdue, follow-up and All. A first request may
omit its date, resolved inside workflow to OS-local civil today using pinned
chrono. Continuations must carry the returned canonical `as_of` date. Counts
cover the whole checked snapshot, independent of filter/cursor/limit1–200.
Unfinished due dates strictly before `as_of` are overdue; follow-up dates on or
before it are due. Date counts may overlap; Completed has neither signal.
Entries retain exact complete Actions and same-snapshot dependency states, in
their stored order; missing is explicit. No status/priority is mutated or ranked.
One query validates every retained Action while keeping scalar target states and
at most limit+1 full records, then returns one checked page. Different pages are
fresh observations, not a frozen membership claim. Current-evidence fences cover
counts and entries; no vault/provider is required. Clients validate the prepared
query/reply through this workflow contract and gain no persistence access.

[Identified completion](src/action_completion.rs) exposes `App::complete_action`
and `CompleteAction`/`ActionCompleted` through AppWorker. The explicit user request
binds one nonnil operation UUID and the whole displayed unfinished Action record.
Full CAS refuses a changed baseline. Completion preserves content and immutable
approved origin, advances one revision, clears Waiting and records monotonic
completion time. It never infers that a message was sent or real work performed.
Completed work stays completed; new related follow-up work uses a new approved
Action with `follows_up`. There is no AI Complete tool or direct client storage path.

The narrow WorkStore transaction holds write exclusion while workflow publishes
one checked immutable ordinary completion receipt in the existing recovery folder.
Only then does SQLite settle the record and receipt atomically. Publication after
the rename boundary or failed settlement after publication returns uncertainty,
fencing current application reads and already-held AI tools. Exact retry or startup
re-syncs checked evidence and reconstructs completion, preserving newer Completed
work and refusing incompatible origins/forks. Startup imports approved snapshots
first, then completions before credentials/model loading and AppWorker Ready.
Terminal replay precedes fresh eligibility and never refreshes time or republishes.
Admitted Complete drains during shutdown; its outer command ID must equal the
typed operation UUID. Fresh publication currently requires macOS; other platforms
refuse before effects. [CLI acceptance](../brn/README.md#manual-action-acceptance)
exercises full request/retry through this boundary.

Shared reference validation uses explicit managed note UUIDs for person/project,
source and thread labels. Newly added references need captured source proof or
exact same-draft managed bytes; retained historical references stay readable.
Action dependency and parent graphs are checked separately against the reviewed
after-state and checked Store records. Fresh creation and ordinary approval run
these checks; approval never silently captures new source authority.

## Tentative review findings

[Findings](src/findings.rs) retain operational review work in brn.sqlite.
`CaptureFinding` uses a fresh duplicate-UUID observation or an exact unresolved
saved-link request. It captures whole-file fingerprints and original UTF-8
occurrence/definition quotes through coordinated saved-source APIs, rechecks proof
and current-evidence fences, and admits no editor, proposal or Markdown change.
Identity evidence retains two distinct observed paths without claiming the
inventory is complete. Existing request replay returns immutable retained work
before current-vault checks, including after closure.

`Findings`, `Finding` and exact-stamp `CloseFinding` remain available without fresh
vault evidence. `InspectFinding` retains the original record and separately reports
Unchanged, Changed or Unavailable full proofs; it checks the bound vault before
reading paths. It never substitutes another vault/path, guesses a quote anchor,
automatically closes an issue or treats unchanged evidence as proof the issue
still persists. Admitted capture/closure drain on AppWorker shutdown. Correcting
knowledge still requires a separate exact approved proposal. Native queue controls
and later scheduled/semantic detection are subsequent slices.

## Typed proposal review foundation

[`knowledge`](src/knowledge.rs) exposes saved-current `NoteIdentity` inspection and
`PrepareNoteIdentity`, a read-only full-source preparation command. Explicit
nonnil note/proposal UUIDs and a title produce a complete ordinary Replace
`DraftRequest`, including exact before/source proofs. Preparation creates no
proposal/editor record and writes no vault content; existing CreateProposal and
exact approval admit/apply the reviewed bytes. Already identified or unsupported
metadata refuses reassignment. Ordinary fresh replacement preserves a recognized
target ID; direct manual Save and exact Undo retain their byte semantics.

`IdentityInventory` scans and reads fresh saved Markdown across current and archive
paths, including unmanaged notes. It reports duplicate UUID paths and incomplete
metadata/file inspection instead of trusting index timestamps or guessing.
`ResolveNoteIdentity` returns explicit Unique/Absent/Ambiguous/Incomplete with
observed matches/issues; unreadable evidence cannot certify uniqueness or absence.
These observations are not an atomic vault snapshot; later durable work still
captures and checks exact source versions. `EvidenceNote` opens explicit saved
source/history text, including top-level archives, through the separate read-only
`EvidencePath`. It applies existing bounds, UTF-8 and non-symlink checks without
opening an editor or changing current mutation authority. All three commands
respect unresolved Save/application fences.

Refresh reads full saved bytes, including archives, and derives UUID/classification
into disposable BRNI V3 rows. Optional `brn_kind: knowledge|source` and
`brn_state: current|history` default to current knowledge; top-level archives
always count as history. Malformed managed identity, classification or provenance is reported in
`RefreshReport.unreadable` and excluded from all scoped queries. Unreadable
evidence files/folders are reported and stale rows removed; unknown subtrees
keep identity resolution incomplete without blocking readable current notes.
Equal size/mtime never substitutes for bytes; equal hashes retain passages/vectors.
Scopes filter before ranking/limits, then returned passages and list rows recheck
exact saved class/hash/quote. Explicit EvidenceNote still reads malformed original
text. Queries never stamp metadata, open editors or mutate sources. AiTools maps
`brn_ai::ReadScope` explicitly to KnowledgeScope for the same scoped search/list/
read checks. Existing read calls default to Current. Owned chat/Rewrite leases
forward scoped calls and retain authority until every blocking read drains.
Native browsing/search controls use the same scoped commands; non-current
openings expose full read-only evidence while direct editing keeps its existing
authority.

AI passage/read/list results also carry required `NoteFacts` derived from those
same complete checked bytes: optional canonical managed UUID, full-note SHA256,
independent Source/History flags and explicit unknown conflict status. All scope
includes mixed classifications, not an inferred class. Read text truncation does
not truncate its hash proof. These facts add no extra scans or file reads, no uniqueness
claim, and no approval or semantic truth authority.

`NoteProvenance` reads durable `brn_provenance` citations from ordinary Markdown,
then freshly resolves each source UUID across current/archive evidence. Matched,
Changed, Absent, Ambiguous and Incomplete outcomes retain the saved exact quote;
paths/hashes never substitute for logical identity. `CaptureCitation` binds a
caller-supplied saved hash and UTF-8 byte range to full coordinated source bytes.
`PrepareNoteProvenance` adds captured citations to complete ordinary Replace
review input, preserving old references and all unrelated target bytes. It
creates no editor/review record and refuses self-source preparation. All three
commands respect unresolved application/Save fences.

Archived evidence can supply read-only proposal source bindings; writable
destinations remain current VaultPath. Fresh normal approval checks newly added
or changed citations against unique identities, exact quotes and captured source
fingerprints, including full edits/Rewrite. Unchanged historical references need
not still match a current source; exact Undo and completed replay preserve their
existing authority. Sources remain checked throughout application/recovery.
Approved quotes survive index loss and inspection from a fresh operational store
with only the copied ordinary Markdown. Actual session Delete qualification and
native provenance convenience follow their roadmap slices.

`NoteLinks` inspects ordinary CommonMark inline/reference links in saved body
bytes, using Store's exact frontmatter body offset and pinned markdown 1.0.0.
Occurrence/used-definition proofs retain full-note UTF-8 ranges and exact quotes.
Contained relative Markdown paths and stable `brn://note/UUID` targets resolve
from a fresh complete identity inventory, with explicit unresolved/non-note/
external outcomes. Sources retain UUID/hash and separate identity certainty;
targets recheck saved bytes before reporting resolution. Inspection respects
current-evidence fences, never opens an editor/writes Markdown or starts network
work, and survives restart/index loss from ordinary Markdown alone. It refuses
4096-link/4 MiB destination-and-quote overflow instead of silently truncating.
These are observations, not a transactional vault snapshot.

`PrepareNoteLink(LinkRequest)` returns complete ordinary Replace review input
for one additive stable UUID link. The current consumer and saved target must
already have freshly unique managed identities and valid metadata; source/history
targets are permitted. The request binds the selected target's full hash, and the
draft captures both full source versions. Preparation preserves every original
byte, escapes the literal single-line label and confirms the exact appended AST
link outside unfinished code/HTML. Self-links and existing resolved links are
refused. No proposal or editor is admitted and no vault bytes change.

Fresh ordinary approval rechecks newly introduced stable UUID targets after
review edits or Rewrite. A saved target needs its exact captured source binding;
a target created/replaced in the same complete draft uses the exact reviewed
bytes and normal destination/before proofs. After-state identity inspection
accounts for filesystem namespace aliases and refuses absent, removed, ambiguous
or invalid targets before Applying. Unchanged historical UUID links are compared
without the public extraction-output caps; exact Undo and completed replay retain
their authority. This convenience command prepares links to saved targets; it
does not coordinate cross-proposal creation dependencies.

`Relationships(RelationshipRequest)` refreshes saved metadata, uses one fresh
identity inventory and rebuilds disposable explicit-link and inferred-provenance
edges. Each unique, eligible managed endpoint retains UUID/path/full hash; exact
source link occurrences/definitions or target provenance quotes remain attached.
Repeated proofs coalesce by directed endpoint pair and origin; self-links and
external/unmanaged targets do not create note-to-note edges. Inferred provenance
is a candidate, never an approved durable relationship. Changed/ambiguous or
incompletely inspected identities cannot become guessed edges. Endpoint bytes
are rechecked before caching; issues and duplicate paths remain visible.

Pages contain 1–200 edges, a matching total and offset, ordered by source path,
target path and origin. Existing scopes filter both endpoints before pagination;
Current is default, and All explicitly includes cross-scope connections. Every
workflow query derives from saved Markdown offline rather than trusting an old
target resolution. The cache is neither authority nor a transactional vault
snapshot. No AI, network, proposal admission or Markdown write occurs. Native
relationship controls follow separately; graph canvas
remains its later roadmap stage.

`proposal_source(path)` / AppWorker `ProposalSource` return the complete saved
text and trusted file fingerprint for initial review composition. The capture
uses current-evidence fences and the same visible Markdown/path/file checks as
proposal creation. It creates no editor/proposal record and writes no vault
content; creation subsequently checks the captured version. This full-note query
does not substitute a truncated AI read-tool excerpt for the proposed before text.

`proposals::DraftRequest` supplies typed Create/Replace/Trash intent. Workflow
captures trusted vault/parent/before identities and exact bytes, checks expected
source fingerprints and refuses occupied or aliased destinations. Creation replay
precedes fresh-vault checks and returns existing edited review work; a conflicting
initial payload cannot reuse its UUID. Review text/comments are operational work,
never current vault evidence.

Ordinary asset members use additive `CreateAsset`, `ReplaceAsset` and `TrashAsset`
variants in this same family. `ProposalAsset(path)` captures a complete fresh
fingerprint for a visible contained current non-Markdown file; it returns no
payload, note identity or retrieval evidence. Workflow captures exact before
bytes itself. Each payload is at most16 MiB, with32 MiB total before/candidate
bytes, alongside unchanged Markdown and operational bounds. Only the new raw
payload fields use canonical padded base64; legacy Markdown serialization remains
unchanged. Parents must already exist. Approval, recovery, repair and Undo use
the same held-root/parent, exact byte/identity, single-link, exclusive namespace
and durable receipt protections. Assets have no text editor or AI tool. Rewrite
receives ordered full proof summaries and null asset text slots; the saved
capture still binds complete immutable bytes. This prerequisite adds no Office
conversion, binary Source preservation or original-copy deletion authority.
The existing Action-bearing Undo refusal remains; asset/Markdown-only whole
Undo and scoped asset Trash restore use the existing inverse family.
`DraftRequest::action_changes` admits exact typed Action Create/Replace members,
including mixed Markdown/Action drafts. Replace fixes the complete checked current
record; edits cannot alter its baseline or immutable origin. The existing budget
is 64 combined members and 8 MiB. Only file/source-free work may omit vault binding;
Markdown/source work requires the exact vault. Completed Actions cannot be edited
or reopened through these members. Owned Action Rewrite changes review only;
Action Undo remains refused pending its inverse contract.

Checked ordinary recovery supports already-typed Action snapshots without
inventing a vault for file/source-free work. Bound and mixed snapshots keep exact
vault checks. A retained terminal Applied mirror restores complete operational
state through Store; unfinished zero-file intents settle NotApplied, preserving
review comments and never treating empty file proofs as Applied evidence.
Restart does not issue a fresh no-effect certificate. Temporary retirement checks
ordered Action kinds/IDs/full Replace baselines while permitting candidate edits;
foreign bindings remain retained. Source-free execution deliberately prepares an
empty file-proof vector, then joins exact Action writes and the whole receipt in
the existing Store transaction. Full Action CAS is checked before file effects
and before publishing an Applied mirror, with transactional CAS retained at
settlement. Refusal after mixed file effects stays Uncertain and fences current
reads. No standalone AI mutation path or dashboard is introduced.
Ordinary approval recovery persistence currently requires macOS, including
source-free Actions. Other platforms retain source-free Draft creation/replay but
refuse approval before Applying admission; they are not qualified application routes.

Full-text edits, anchored or whole-proposal comments, explicit reattachment,
rejection and imported captured Rewrite results use one version. Changing an
anchored target leaves its old range unresolved; late results after newer edits,
comments or rejection fail. AppWorker owns and drains admitted review mutations.
Read/list/edit/comment work remains available without current vault access.
`validate_review_edit` provides pure full-result validation using Store's bounded
typed rules before the frontend queues an edit. Owned AI Rewrite uses the
lifecycle below. Native exact individual/captured-group approval and activity use
these same DTOs; native Undo/repair confirmations use the existing exact previews.

The current-evidence fence covers pending/Uncertain Save and proposal journals.
It refuses tools, note/list/search, refresh and embedding, including startup and
model activation; retained tools cannot read through the fence. Settling a Save
cannot clear a proposal's uncertainty. Pending proposals also refuse new Save,
Save Copy and reload, while keeping editor recovery and existing proposal review
readable.

`proposal_apply` exposes exact individual and captured-group approval through
AppWorker and the CLI. Preflight checks reviewed vault/parents/targets/sources and
all editor namespace/identity aliases. It fences retained read leases before
intent. All Create/Replace members are staged before effects; Trash retains its
original. Complete prepared proofs reach SQLite and a bounded, hash-checked
ordinary recovery receipt before coordinated exchange/exclusive installation.
Whole installed/displaced proofs and required file/directory sync precede one
Applied result. No-effect refusal is certified only in the fresh path before a
namespace attempt; interrupted/mixed proofs remain Uncertain. Replay writes
nothing. Group membership and versions are explicit; each proposal is a separate
unit and later members stop on refusal. Later editor typing keeps its old baseline
and buffer as a visible conflict until explicit reload.

Recovery receipts remain in the data folder across SQLite backups. Every startup
imports newest terminal evidence before earlier history and unresolved work,
then classifies pending file proofs before binding current evidence. Trusted
historical completion preserves subsequent owner bytes. Reconciliation checks
current sources for incomplete work, recognizes sources replaced/trashed by that
proposal, and never repeats installation. Approved annotation cleanup covers
current/prior journals, ordinary snapshots and compatible proof-checked temporary
snapshots; unexpected occupants remain retained. Native exact approval and
reconciliation use these same workflow rules. `ProposalRecovery` returns small
checked pending/uncertain summaries, validating one journal at a time;
`ProposalApply(UUID)` loads only the identified full historical snapshot. Paged
native Activity never retains every historical proposal body. Explicit CLI
`applies` remains the full operational inspection command.
Activity summaries count both approved Markdown and Action members. Their
`changes` field lists note paths; the identified full application snapshot retains
complete Action identities/data/baselines without loading every body into a page.

Knowledge-bound approval operations also retain an immutable
`.brn-apply-<operation>.inbox-capture` companion. Its canonical hash-checked payload
holds the exact approval request, Knowledge-binding digest and genuine full
`InboxActionJob`, bounded at1MiB+4096 encoded bytes. Existing format1 receipt bytes
and occupied equal inodes remain unchanged. Capture publication precedes effects;
equal companions are checked/synced and different occupants refuse. Canonical
companions are validated even without a receipt, but valid prepublished orphans
remain inert. Unknown/corrupt names or content fail closed. Older builds reject
the new companion suffix; keep the qualified reader when reopening these data.
Retained evidence is never removed to permit a downgrade.

Startup imports the genuine capture and approval together in one Store transaction.
An older receipt may gain its companion from checked retained SQL; missing evidence
is never synthesized from current Source bytes or provider defaults. Fresh/older
operational recovery creates no session/turn and cannot resubmit an issued
historical analysis UUID. Comment cleanup, repair and newer review retain these
bindings. This uses the existing approval family and grants no new approval or
filesystem authority. Qualification/integration and remaining acceptance are
recorded separately in the current checkpoint.

## Owned AI Rewrite

`proposal_rewrite::RewriteRequest` binds a job UUID, exact proposal stamp, explicit
provider/model, low/medium/high effort and event generation. AppWorker's
`StartProposalRewrite` replays history before current-vault/account access, then
preflights the current AI vault and read tools. Source-free Action reviews have no
file binding and need no prior editor visit; captured file/source bindings must
match exactly. Fresh admission captures the complete
proposal/comments on the checked owned AI lane. `ProposalRewrite` reads safe
job history offline. No automatic provider/model fallback or retry occurs.
Running replay returns AlreadyRunning history, preserving the original owned
generation; it does not subscribe a new presentation generation to that job.
Clients use `RewriteRequest::check_replay` for exact job/provider/model/effort
correlation; presentation labels are never operational provider keys.

Ask and Rewrite share one active owned job, cancellation registry and read-tool
lease drain. Stop withdraws queued Rewrite before admission or cancels active
work; provider disconnect and shutdown join retained reads before releasing
credentials/ownership. A consumed successful result can win a later Stop.
Running jobs become Interrupted at restart without resubmission.

The Rig adapter bounds full capture and buffered output to 50 MiB encoded JSON;
it never truncates the full proposal or emits raw Rewrite deltas. Strict output
contains title, every note member's full text/null and ordered full `action_data`
for every Action member. Each Action requires all14 fields, including explicit
nulls; missing, duplicate or unknown fields refuse. Legacy Markdown results may
omit the empty vector and retain their old hashes. Immutable Action kinds/UUIDs/
full Replace baselines and source bindings never come from provider output.
WorkStore additionally enforces Action domains, complete member counts, decoded
1 MiB/note and 8 MiB aggregate review limits. Completed candidates refuse. Malformed,
partial or oversized results settle a safe failure without editing. Validated
result and terminal job commit together against the captured stamp/hash; later
review changes settle Stale. Rewrite changes operational review only. Temporary
comments/prompt/raw output are not copied into chat or durable job metadata.

`activity::ActivityRequest` projects successful Applied journals into readable
history through AppWorker and `brn activity list`. Default pages contain 20 entries
(1–100 allowed), newest approval time then operation UUID first; an existing
Applied UUID is the exclusive older-page cursor. Entries include the title,
Create/Replace/Trash path summary and operation/proposal/group/session identities.
The recorded time is approval admission, not an exact completion time. Note bodies,
comments, file proofs and raw tool transcripts are absent. One checked journal is
decoded at a time, keeping bodies out of the full-history ordering metadata.
Drafts/refusals/uncertain work cannot claim a successful durable change. Historical
activity stays readable when current evidence is fenced or its vault unavailable,
and ordinary recovery restores it without repeating old effects. Native activity
presentation uses the same paged projection and exact journal snapshots.

## Explicit Undo and Trash restoration

`preview_proposal_undo` returns the complete inverse of one Applied operation as
operational review data; it does not claim current file eligibility. `undo_proposal`
uses a fresh caller UUID and an immutable whole/single-Trash scope. The shared
AppWorker `PreviewProposalUndo` / `UndoProposal` commands return a typed preview or
existing `ProposalApplied` receipt. Admitted Undo drains on shutdown; AI has no
direct Undo tool. Whole Undo never silently selects a subset. Explicit single
Trash restoration can preserve later edits to other members of a mixed operation.

Fresh preflight checks the same bound root, parents, targets and editor aliases as
approval, plus every exact retained original. Preparation flushes and borrows its
inode instead of copying it, preserving original bytes/mode/ACL/xattrs. The same
whole prepared proof, durable mirror, coordinated installation, strict settlement
and recovery protocol applies. Unknown partial work stays fenced; reconciliation
and UUID replay never repeat namespace effects. A clean editor whose baseline/text
exactly matches the restored original may remain bound; dirty or incompatible
recovery refuses. Tokens, generations and queued later typing remain untouched.

Activity includes the source operation and optional original Trash member index.
The existing 64-member/8 MiB proposal limits apply. Old retained Trash is eligible
while its proofs hold; no timer/count silently purges it. CLI commands are described
in [brn](../brn/README.md). Native Activity shows the full historical inverse and
Applied snapshots expose restoration by original Trash member index. Current
eligibility is checked only on explicit confirmation. Native owner usability
acceptance remains pending.

## Explicit interrupted-operation repair

`preview_proposal_repair` returns the full approved draft, exact current
Before/Applied member phases and a capture hash. `repair_proposal` binds a fresh
attempt UUID, that hash and Finish/Restore direction. AppWorker exposes
`PreviewProposalRepair` / `RepairProposal` with typed previews/receipts; admitted
repair drains on shutdown. CLI [commands](../brn/README.md) use the same boundary.
AI has no direct repair tool.

Admission checks the bound vault, complete file pairs and retained editor aliases.
Finish checks sources against arbitrary current phases and exact Action baselines,
installs only remaining approved changes, then joins Actions with the whole receipt.
Restore returns applied members to exact originals while preserving unrelated
external sources and competing Actions. A durable repair mirror precedes namespace
effects. Each coordinated move proves its current pair, uses exclusive installation
or exchange, flushes surviving files and parents, and proves the complete endpoint.
Unknown destinations/staging, dirty aliases or changed capture refuse. Comments
are deleted only on Applied completion; restored Draft comments remain for review.

The original first Uncertain receipt/proofs remain immutable until known whole
settlement. Up to 64 bounded attempts retain their exact requests and outcomes;
the latest outcome settles with the whole receipt. Replay, startup and reconciliation
never repeat namespace changes. Known terminal repair requires every exact Before
or Applied pair, including staging proofs; older/missing SQLite restores the same
checked history. Temporary repair snapshots retire only against compatible checked
canonical history. Editor baselines/generations and later queued typing stay intact.
Native repair offers explicit full Finish/Restore confirmations, displaying the
observed phases, complete approved draft and matching recorded comments. Each
attempt retains its exact hash/direction and reports the actual optional outcome;
failed attempts remain inspectable without automatic retry. Native owner
usability acceptance and physical power-loss qualification remain pending.

## Explicit model installation

[`models`](src/models.rs) exposes the pinned multilingual source, approximately 129 MiB cost
and destination. `prepare_model_download(consent, target)` persists
`model.download_decision.2c4055b12046f11709e9df2c122e59ffbdc2f900` as approved/declined.
Earlier generic consent is retained and cannot authorize or suppress the new offer.
Only a **fresh explicit
approval** creates a non-cloneable `ModelInstallRequest`; startup, search and
persisted approval never download. A later explicit action can override decline.
The fresh default target is `<simple-data-dir>/models/multilingual-minilm-l12-v2`;
saved and explicit model paths retain precedence, and old asset directories remain untouched.
Targets cannot overlap
the vault or a repository.

The worker consumes the request's synchronous `install(cancel, progress)` on an owned
blocking job, not on the GUI/application lane. It owns job lifetime,
cancellation and events: report `ModelDownloaded` on install success, drain
and detach idle chat/tool handles, then call `App::activate_model` on the
application lane. External tool handles cause `ToolsBusy`, rather than swapping
an active snapshot. Activation loads once, replaces both shared adapters and
invalidates vectors by model identity; subsequent bounded `embed_pending`
calls rebuild them. Emit `ModelInstalled` only after successful activation.
Invalid installed models remain `ModelInvalid`, not model absence.
Worker indexing is scheduled only with a loaded model and an available bound
vault, including between batches. Startup and activation still succeed without
a vault; missing indexing prerequisites do not produce a later correlated
failure. Binding an available vault or refreshing a restored bound vault resumes
bounded indexing. Actual embedding/index errors remain explicit `Failed` events.

Headless/default builds are keyword-only and ignore any saved `model.directory`
without changing the setting or assets. Only an explicitly supplied model
directory or Download action returns typed `SemanticUnavailableInBuild`
(`SEMANTIC_UNAVAILABLE_IN_BUILD`). Unsupported Download does not change consent,
and these builds do not offer an automatic download prompt. Explicit decline
still persists without network. Native builds enable `native-retrieval` and
continue to honor saved model directories and fresh consent.
The CLI and desktop simple editing/recovery/reads/history/AI actions use
AppWorker. Old and mixed authorities remain refused without migration.

## Simple manual Markdown editing

[`editor`](src/editor.rs) provides `open_editor`, `recover_editor`, `save_editor`,
`reload_editor` and `reconcile_editor` through AppWorker. `EditorView` separates
fresh saved bytes/fingerprint from the protected WorkStore buffer and exposes
conflicts and pending operations. Recovery remains readable when the vault is
unavailable. UTF-8, BOM, line endings and frontmatter remain exact; empty notes
are valid and text is limited to 1 MiB.

Save journals the generation-bound input before coordinated file work. The
shared private macOS adapter validates root/parent/file identity and vault
ownership, preserves attributes, uses durable sibling staging and atomic
exchange, and never recreates a missing original. Save Copy installs exclusively
at a distinct unused Markdown destination. A copy does not resolve or rebind the
original. Required coordination/durability failures have no weaker fallback.

Replay never repeats filesystem writes. Reconciliation checks prepared,
destination-parent and installed/displaced identities; matching text alone
cannot prove application. Unresolved saves fence current knowledge reads and AI tools.
Applied completion preserves later typing. One recent Applied recovery pair and
compact settled receipts remain after identity-proven artifact cleanup;
unexpected artifacts and unresolved payloads stay protected. Reload binds the
exact reviewed disk observation and requires confirmed discard of dirty text.
Already-admitted recovery/save/reload/reconcile commands drain on shutdown.
Stage 1 automated checks passed; native usability remains pending in
[status](../../docs/status.md).

## Reversible session organization

`ConversationSummaries(filter)` exposes explicit Active/Archived/All history and
checked lifecycle versions without changing canonical conversation or turn JSON.
The compatibility `Conversations` query still returns all sessions. Archive/Restore
uses `SetConversationLifecycle` with an operation UUID and exact expected stamp;
its receipt separates the original transition from current lifecycle state.
Replaying an old Archive after Restore never archives again. Creation/activity
times, budgets, captured outcomes and evidence remain unchanged.

The private chat lane serializes these critical mutations with actual AI admission
and refuses new transitions while work is active or draining. Exact read-only
operation replay remains available. Store transactions independently fence fresh
Ask, Inbox reservation/turn and Rewrite admission; archived sessions require
explicit Restore. Inspection, proposal approval and recovery remain available.
Automatic inactivity archival, Delete and preferences are separate behavior.

## Owned application and chat lanes

[`AppWorker`](src/app_worker.rs) is the frontend handle:
`start(data_dir, AppConfig)` spawns before any SQLite open, vault scan or model
load. Opening errors arrive as `Failed`; startup emits optional `Restored` then
`Ready`. `submit(uuid, AppCommand)` and `try_event()` /
`recv_event_timeout(timeout)` use `(uuid, AppEvent)` results. Bind, selection,
local status, editor/recovery/save/reload/reconcile, notes/search, history,
model prompt/consent/progress, account commands and terminal errors all use this
seam. Recovery acknowledges SQLite's unfinished work; explicit Save writes Markdown.

The private [`ChatWorker`](src/chat_worker.rs) owns its runtime, attached
`ChatStore` and `Arc<Auth>`. Keeping admission private prevents frontends from
bypassing App refresh or discovery membership checks. App refreshes before
each **new** Ask, requires an available bound vault and explicit valid
selection, and checks conversation existence locally. Prior UUID replay comes
first: terminal replay is history-only even with an unavailable vault or
selection no longer in discovery; a Running record is `AlreadyRunning`, never
resubmitted. Different payloads/generations conflict. Outer submission UUID
must equal Ask/account operation UUID. Durable replay matches the recorded
question, conversation, provider and model; generation is a transient
navigation correlation, not persisted history.

Conversation projections include known `created_at_ms` and nullable
`last_activity_at_ms`; turns include nullable `started_at_ms`/`finished_at_ms`.
These are Unix milliseconds from checked WorkStore chat transactions. Historical
unknowns remain null. New admission and genuine finalization update activity;
reads, exact replay and startup interruption do not. Recovery does not invent a
finish time, and a nonpersisted partial has no trusted time. Existing
Conversations/Turns/Turn commands carry the same values without a new workflow.

`AppCommand::Turn(uuid)` / `AppEvent::Turn(Option<WorkTurn>)` is an owner-lane
lookup for CLI replay projection. Query Selection explicitly, but use the
recorded provider/model during replay even if current selection is obsolete.
`WorkflowError::recorded_ai_failure` projects the closed persisted AI category
to safe typed errors, rather than frontend wording classification.

One turn is active. Dispatch continues while turn/auth futures await. Stop,
account actions and installer cancellation bypass application work; chat and
transient account events are forwarded independently of scans/loading.
Disconnect fences its provider, cancels and joins only that provider's jobs,
then removes caches after clients/tools drain. Other-provider account work
continues. Cancel Connect is not Disconnect. Explicit discovery is recorded on
the application lane **before** its successful Models event is forwarded.
Account/model operation UUIDs cannot be reused to start another job.

`app::model_history` retains the last 20 earlier terminal text pairs, including
failed/interrupted partials. The chat lane snapshots them after the previous
turn's commit; Running turns are excluded. Rig receives no tool/provider
metadata and omits empty assistant text while keeping its question.
Every chat event includes turn UUID and generation; navigation must filter
stale events without preventing the worker from persisting their conversation.
`Finished` follows the terminal commit. `PersistenceFailed` contains the
in-memory partial and safe error: it does **not** establish saved text.

Installation progress uses the submission UUID. `ModelDownloaded` precedes
activation; active turns and all queued/running blocking tool leases must drain
before idle-only tool detachment. Loading happens once on the app lane.
Cancellation before model publication leaves the old model intact (synchronous
loading itself cannot be preempted). Tools are reattached before
`ModelInstalled`; vectors rebuild in batches of at most 16 between commands.
Indexing progress uses its startup/refresh/download submission UUID.
Activation failure remains visible and reinstalls the prior tool adapter.

Call `shutdown()` and handle its error: it cancels and joins admitted turn,
account and install work before releasing App/owner, including failed terminal
persistence or credential finalization. Repeated shutdown retains the outcome.
Drop also cancels/joins, never delegates to a detached reaper. Stop cannot
guarantee upstream cancellation or no billing; OS-killed processes lose
uncommitted stream text. Private `cfg(test)` external-operation adapters are
not provider features, public registries or CLI flags.

Focused offline checks:

```sh
cargo test -p brn-workflow --test library --test app --test ai_tools --test models --locked --offline
cargo test -p brn-workflow --features native-retrieval --lib --test models --locked --offline
cargo test -p brn-workflow --test app_worker --test chat_worker --test app --locked --offline
cargo test -p brn-workflow --lib simple_worker_tests --locked --offline
```

Native installer/workflow checks use synthetic assets and fake embedding
vectors, not production downloads or ONNX inference qualification.

## Dependencies and features

Only store, retrieval and AI are workspace dependencies. Default builds are
keyword-only; `native-retrieval` enables the explicit local embedding/installer
path. Private macOS files/coordinator types enforce manual Save without a generic
repository or workflow framework. Default and optional native checks remain
separate; see [verification](../../docs/development/verification.md).

## Inbox source review preparation

`PrepareInboxSource` / `prepare_inbox_source` turns one qualified conversion into
one complete Source Create draft, paired with its exact ordinary PNG asset Create
for the visual profile. `CreateProposal` and exact approval retain their
existing lifecycle; preparing/admitting a draft never writes the vault. Imported
frontmatter remains body evidence under a new UUID and `brn_kind: source`.
Portable `brn_inbox_source` provenance is available through `NoteProvenance`.
Source copies retain exact body/proof through edits and Rewrite; separate Action
and opt-in knowledge proposals with captured saved link targets are implemented
below, including paired supersession and tentative unresolved conflicts. Original proof is checked during
unfinished application/Finish repair; completed historical replay and ordinary
recovery do not require processing rows or the original to remain available. Restore
repair may remove a partial source without recreating or deleting the original.


## DOCX inline PNG and explicit interpretation

The first visual profile accepts one ordinary inline PNG at most1MiB encoded,
4096 per dimension and4,194,304 pixels. Complete integrity/EOF and a hard32MiB
decoder-owned allocation budget are checked; animation, meaningful unsupported
cropping/transforms/drawings/charts and extra meaningful images refuse. Exact
alt/title/caption wording and occurrence survive conversion. Source/asset creation
uses the existing whole proposal/apply/recovery/Undo family; no preview effects
or original-copy cleanup are authorized.

`InboxVisualEvidence(source_path)` returns the complete saved Source and asset
proofs plus actual checked PNG bytes after unique Source identity qualification
and final Source reobservation. Its typed `visual_proof` accessor serves client
presentation; inspection is provider-free and grants no approval authority.
`AnalyzeInboxActions` with explicit `purpose: visual_interpretation` and the exact
`visual_asset` uses the existing owned WorkTurn lane and explicitly selected
provider/model/effort. New admission rechecks Source/PNG proofs; the ephemeral
image is not stored in the question/job. The narrow AI route receives no tools
or history. Existing Action/Knowledge purposes keep their capabilities unchanged.

Only a completed bounded exact description/uncertainty JSON result can prepare
`PrepareInboxVisualAnnotation(analysis_id)`. It reconstructs one tentative
Source-only Replace, protecting literal text, metadata and image reference.
Preparation is provider-free; clients explicitly create/review that draft before
separate exact approval. Creation replay preserves original binding and newer
review; historical apply/capture recovery and provider-free Undo retain existing
semantics. Failed/interrupted/malformed results cannot prepare an annotation.
Binary originals remain retained; broader Office/visual formats and meaningful
Office cleanup are unfinished. GUI/live/model/owner acceptance is separate from
automated qualification.

## Opt-in Inbox knowledge consequences

An `InboxActionRequest` with explicit `purpose: knowledge_and_actions` admits
knowledge and Actions through the same `AnalyzeInboxActions` command and owned
Ask/WorkTurn. Omitted purpose remains Actions only; old stored questions and V14
canonical bytes replay unchanged. `InboxActionAnalysis` exposes the retained
purpose, whole Source, turn and independently reviewable group. It always marks
remaining semantic review; no original deletion or completeness is implied.

Only that bound semantic turn enables `propose_knowledge`. One call supplies a
Current destination, complete candidate Markdown without managed identity, selected-Source
exact quote text with optional 1-based body occurrences and ordered additional `source_paths` (0–63 paths; legacy omission
means empty), plus optional `supersedes` Current predecessor path. Workflow captures full saved target proofs after the selected Source;
explicit Source/History evidence keeps its scope and pending drafts are not saved
targets. Workflow assigns/protects identity and exact saved quotations in
`brn_provenance`; Rust computes UTF-8 byte ranges within the retained saved body.
Omitted occurrence requires a unique match; overlaps count in start-byte order.
Missing text, ambiguous matches and invalid occurrences produce fixed typed
refusals. Metadata and normalized/guessed wording cannot establish a quote.
Rust mints distinct proposal/note UUIDs from the owned analysis and exact serialized
semantic intent before adding metadata or observing mutable evidence. Exact input
retries reuse the original capture and return newer owner review; changed intent or
another analysis creates different identities. IDs grant no approval authority.
It rejects model-injected identity, Source/History/invented provenance, stale/ambiguous Source and occupied/uninspectable new identity. Actions,
knowledge and conflicts share the existing20-consequence cap. Creation replay preserves later
review edits and original ordered proofs without re-reading lost sources or
calling a provider; changed intent takes fresh capture and validation under a new identity.

Optional `inbox_knowledge` metadata in the existing typed proposal binds analysis,
identity, Current classification and selected citations across edit/Rewrite.
Exact approval, application, recovery and Finish recheck Source/identity authority
and captured stable-link target proofs against the complete identity inventory; only
the operation's exact prepared object may already occupy its new UUID. Public
identity reads stay fenced during unresolved application. Existing recovery,
Activity and Undo remain the application mechanism. Ordinary Ask, Action-only
analysis, readonly answers and Rewrite never acquire the new capability. All
callbacks share admission/cancellation/draining; clients own no persistence or
provider implementation. Saved links and relationships use the existing derived
queries and rebuild after index loss. Optional supersedes produces one exact
Create/History Replace pair: a distinct new Current UUID and the predecessor at
its existing path with only brn_state changed to history. Workflow adds a Previous
version link, captures the full predecessor proof automatically second, then
ordered extra targets (0–62). Do not repeat the predecessor in source_paths.
Unmanaged/Source/History/ambiguous predecessors refuse. Current content remains
editable with immutable citations/footer; History bytes remain protected through
edit/Rewrite. Exact approval also requires the footer to parse as a body link.
Creation replay reuses retained before_text/proofs ahead of fresh files. Apply,
recovery and Finish qualify predecessor identity against its exact before or own
prepared historical object; equal bytes or another inode do not suffice. Current
retrieval excludes that history; explicit History and existing relationships/Undo
remain shared and rebuildable. Safe original-copy deletion remains the next
Stage7 slice.


## Tentative Inbox conflicts

Only the owned KnowledgeAndActions turn admits `report_conflict`. It captures
one Open Finding with the analysis UUID, exact title/summary and two ordered
saved body quotations: selected approved Source first, then a distinct managed
Current knowledge/Source path. Whole proofs and quotation wording/ranges remain
immutable. The tool supplies quotation text and optional 1-based body occurrences;
Rust resolves exact ranges and mints a stable UUID from the owned analysis and
exact candidate intent. Identical input retries reuse original proof/closure;
changed intent or a different analysis has another identity. No cross-task
semantic deduplication or approval authority follows from that identity.
Fresh capture requires complete unique identities, exact selected
Source version, nonhistorical effective scope and UTF8 body ranges. It creates
no note, Action, relationship or proposal approval. V9 Findings in V14 WorkStore
protect capture, original request hashes, startup/backup and history-only replay;
identical callback retries preserve later closure even after saved files vanish.
Findings remain retained after failed/interrupted turns.

An intake-bound investigation may capture a Finding once its exact Source
prerequisite is Applied. The unchanged capture keeps `source: None` and its
canonical omission, retaining its intake binding; Workflow obtains the historical Applied installed
fingerprint from Store, rechecks current prerequisite/original/assets/identity,
and requires full saved fingerprint and bytes to match. Equal-byte replacement
at a new inode does not refresh authority. Pending intake is refused. Historical
Finding reads, explicit closure and exact callback replay retain that proof after
Source changes/loss/Undo; replay performs no inference or new capture. Existing
Finding, capture/question and recovery formats stay unchanged. Private-intake
original cleanup remains unsupported.

`InboxActionAnalysis.findings` includes every retained conflict, including closed
work. Existing Needs Review handles exact stamped Resolve/Dismiss; closure changes
operational review state and conveys no knowledge correction or authoritative resolution. The
shared20-consequence admission cap includes Action/knowledge proposals and
conflicts; original creation replay remains available at capacity.

`NoteConflicts` / `note_conflicts` provides complete Open conflicts for an explicit
saved managed path and scope (Current default). The active exact vault, stable
identity, effective scope and full saved version bind its cursor; a closed
matching anchor can still continue pagination. Match retained path or UUID so
changed/renamed notes retain visible disagreement. Every entry includes immutable
proofs and separate fresh Changed/Unavailable observations. Ambiguous/incomplete
identities cannot appear unchanged. Whole encoded pages exceeding1MiB refuse
without clipping; smaller pages retain full records and a valid continuation.
The result adds `NoteFacts` from the same full saved proof/classification, with
Known open count bound to its existing managed UUID and `SourceVersion`. The count
covers retained open findings across pages, including stale evidence. Basic AI
search/read/list remain Unknown; errors never become zero. Known zero establishes
no retained open finding for that lookup, never consistency, truth or a winner.

Ordinary Ask's `read_conflicts` stays on the owning AppWorker read lane; its opaque
cursor is the serialized shared DTO. Instructions require disclosure of unresolved
conflicts/stale evidence and do not treat incomplete pages/errors as absence.
Thin CLI and native analysis-to-Needs Review navigation use these shared commands.
Synthetic transport/worker/headless tests do not establish native usability, live
provider compliance, semantic completeness or original-copy deletion acceptance.

## Complete original review evidence

`InboxReview` / `inbox_review` obtains the complete checked retained original
review manifest from one Store snapshot and freshly observes the exact owned
original. The response contains full review records, their canonical digest,
original availability/text, and `needs_semantic_review: true`. Whole encoded
responses over64MiB refuse. Missing/changed copies remain visible rather than
being mistaken for intentional removal. Saved Source freshness/identity and
semantic completeness are not qualified by this read. No files, approvals or
review states change; exact confirmation, recoverable removal and its recovery
remain subsequent gates. UI and CLI use the shared AppWorker boundary.

The private typed `ai_behavior::TaskInput` assembles full captured Inbox-analysis
and Rewrite input. Static agent instructions/tool policy remain in `brn-ai`;
BRN domain context, evidence validation and application authority stay here.

`PreviewInboxRemoval` / `preview_inbox_removal` qualifies one approved Source
that still preserves the exact original. The selected full Applied approval,
original bytes and freshly observed saved Source bind the bounded digest. Rust
reconstructs the existing conversion and verifies the complete saved body,
managed UUID, Source classification and original provenance. Owner metadata
Save, a new inode, relocation, History and a visible archive path may qualify;
changed body/identity/provenance, duplicate identity or incomplete inventory refuse.
Actual unresolved Save/Apply/completion authority still fences current reads.

Failed or pending analyses, processing, consequence drafts, running Rewrite and
later derived-note edits do not block preservation or alter its digest. A stale
alternative Source does not veto a valid selected Source. Unsaved editor work is
retained independently and never substitutes for saved evidence. The selected
original/journal/full Source proof is rechecked before returning. The complete
review remains available through `InboxReview`; it is independent of cleanup
admission. `needs_owner_confirmation: true` never grants removal authority.
Explicit confirmation must bind this same witness and requalify it. The following
original-copy lifecycle implements that command; native controls remain later work.

## Recoverable original-copy lifecycle

`RemoveInboxOriginal` accepts a complete typed request: operation/capture UUID,
exact preview digest, version1 true exact-copy confirmation and, after a prior
cycle, the current settled Restore UUID/record digest. Workflow rechecks the same
approved preserving Source before the exclusive move. Catalog, capture receipt,
Source and semantic review work remain retained. `RemovedRetained` identifies the
operation-owned exact copy; each read rechecks it and the vacant ordinary endpoint.
No automatic removal, disposition change, permanent purge or AI cleanup tool exists.

`RestoreInboxOriginal` binds the current settled Remove UUID/digest, exact retained
bytes/inode/namespace and vacant destination. Source freshness is independent of
restoration. Both commands return immutable original receipts on exact replay
before fresh admission and never repeat an old effect against a recreated path.
AppWorker correlates effects by operation UUID and drains admitted work at shutdown;
the owner-operated CLI is a thin bounded request/reply adapter.

Checked Store records and ordinary mirrors share the existing private family.
New lean certificates use explicit format2; format1 historical bodies, omitted
empty-history fields and digests remain unchanged. Historical lookup returns an
explicit legacy variant. Bootstrap imports exact catalog identities/genuine legacy
jobs before approval companions and exposes invalid certificates as startup errors.
It never fabricates chat or processing state. Intake imports mirrors atomically,
computes causal heads once and visits checked SQL records to repair missing mirrors.
Startup only acknowledges exact already-performed moves; untouched intents remain
pending, and any head fences capture-stage installation. Unknown/conflicting files
remain retained. Native/live/owner qualification is separate from automated tests.

## Exact portions of long saved evidence

`AiTools` implements `ReadTools::read_note_range` through the existing complete
1MiB vault evidence reader. It validates the requested full SHA-256 before
returning exact UTF-8 bytes, scoped complete-note facts and total length; changed
content anywhere refuses rather than substituting a newer baseline. Current
evidence fences and blocking-read drain leases are preserved through both
application and chat wrappers. This is a checked content snapshot, not a new
persisted store or transactional filesystem snapshot. Metadata-invalid raw access
is not added by this slice. The existing Ask/native chat can use the range tool;
no direct filesystem access is granted to external agents.

### Per-investigation budgets

AskRequest and InboxActionRequest accept optional strict WorkBudget
`{max_tool_rounds, timeout_seconds}` (1..32/1..3600). Fresh omitted default is
8/300; existing UUID omission resolves recorded metadata. Admission records the
budget atomically in the work DB, separately from canonical turn/capture/recovery
bytes. Changed explicit budget conflicts; historical absence stays unavailable.
Historical unfinished reservations cannot silently acquire a budget/restart.
AppCommand::RunBudget queries metadata separately; InboxActionAnalysis includes
it in the inspection wrapper. Rewrite keeps its separate existing contract.

BudgetProgress reports the frozen budget/completed model responses/admitted tool
rounds, first0/0 after cancellation registration. The owned deadline begins at
durable turn admission before authentication; expiry fences the shared proposal
token, emits BudgetStopping, awaits operation and retained blocking read leases,
then persists failed/time_limit_reached. It retains ordinary semantic partial
text/drafts; a late successful operation cannot replace expiry. Already-ready
completion wins a simultaneous deadline. Manual Stop and tool-limit exhaustion
retain their distinct causes. Legacy strict visual JSON remains completion-only.
No local terminal guarantees upstream cancellation or absence of billing.

### Conflict recommendations and selected consequence review

Saved-Source Inbox Knowledge investigations may recommend a provisional preferred
resolution with opposing evidence, reasons, alternatives and uncertainty. Reported
identity, copied recipients, timestamps and preservation approval alone do not
authorize knowledge changes. Findings retain exact opposing quotes/proofs; summaries
and answers may explain a preference. Only exact displayed proposal approvals make
selected Current/History or Action consequences durable. Finding closure is a
separate explicit review operation and never applies knowledge changes.

For the bounded conflict journey use an explicitly captured saved Source. Legacy
report_conflict does not accept intake-only capture, including one whose Source is
already Applied. Existing supersedes creates a new Current successor and protected
predecessor History with a Previous version link. If another selected proposal also
binds that predecessor's original proof, approve it before supersession; submitted
group order is significant. Stale proofs refuse rather than silently rebasing.


### Owner attachment of a Knowledge predecessor

`AttachInboxKnowledgePredecessor` accepts an exact `ProposalStamp` and a saved
Current predecessor path for one supplemental Draft Inbox Knowledge Create.
Workflow captures the complete predecessor, preserves every existing proof,
identity, owner text prefix, comment and citation, and adds the existing protected
History Replace/Previous-version link with one version advance. Already-retained
context is promoted only on exact proof equality. Shared approval target/editor
guards refuse stale parents, occupied destinations and unfinished editor work;
the stable footer must remain readable Markdown. Pending private intake remains
a preparation prerequisite; applying still requires its exact Applied Source.

Original model creation replay reconstructs the original single-Create payload
and original proof order from retained proofs, then checks the unchanged Store
creation hash. It returns the current reviewed record before fresh file access,
including after attachment, subsequent edits and restart/Source loss. Existing
same-shape replay and complete-pair ApplyJournal/recovery formats are unchanged.
The operation does not decide semantic replacement, alter original captures or
close Findings. Only the revised reviewed version can admit effects.
