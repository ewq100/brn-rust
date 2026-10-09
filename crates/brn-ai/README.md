# brn-ai

> Requirements/qualification context (2026-10-07): this README describes implemented behavior, not mandatory limits or acceptance of the proposed replacement. The [owner amendment](../../docs/product/BRN_PRODUCT_VISION.md#owner-amendment--2026-10-07), [reassessment](../../docs/audits/BRN_ARCHITECTURE_REASSESSMENT_2026-10-07.md) and [proposed plan](../../docs/work/active/architecture-reassessment/plan.md) reopen mechanisms. Email enum/literal text is not real EML ingestion; broader conversion, AI draft freedom/budgets and practical reviewability remain gaps. No production behavior changed in this documentation task.

Thin, fixed ChatGPT/Copilot subscription authentication and streamed
chat over Rig **0.43.0**. Contains account/selection DTOs, safe errors, checked
credential storage, owned clients, seven read tools and one separate review-proposal capability. It does not contain
workers, SQL, selection persistence or frontend state.

`answer_with_effort` freezes an explicit low/medium/high choice with the selected
client. It sends route-specific reasoning parameters on the initial request and
tool continuations while preserving ordinary Ask history, provisional text and
read-tool limits. The older `answer` entry point remains a compatibility seam;
fresh application Ask uses `answer_with_proposals` with explicit effort and a
captured workflow capability, without a provider default.

All Ask entry points instruct the selected model to normally answer in the
current question's language, honor an explicit language request, handle mixed
English/Estonian content and preserve original source-quote language. This adds
no language detector or extra call. Synthetic transport tests qualify instruction
delivery and exact bytes; live response-language compliance remains unqualified.

`rewrite` consumes the same owned client and read tools with explicit
`ReasoningEffort::{Low,Medium,High}`. The pinned route determines Responses
`reasoning.effort` or Chat `reasoning_effort`; the selected model is unchanged.
Full input and buffered output are bounded to 50 MiB encoded JSON. Rewrite sends
no chat history and emits only safe tool progress, never raw text deltas. Its
prompt requests full ordered note texts and all14 explicit ActionData fields;
Action identities/baselines/source bindings and approval stay outside model output.
Stop,
transport/schema/tool refusal and over-limit output discard partial text without
application retry or fallback. A consumed completed result can win later cancellation. Workflow
validates strict full-member JSON and commits review work through WorkStore CAS;
this adapter never applies knowledge or persists prompts/comments/results.

Static Ask, Action-review, Inbox-knowledge, visual interpretation and Rewrite instructions/capability
selection live behind the private typed `behavior` boundary. Individual tool
descriptions remain with their implementations; workflow supplies captured task
context through its private typed boundary and retains deterministic authority.
Inbox Knowledge proposal inputs supply semantic candidate text/evidence, without
proposal/note IDs. Workflow mints both identities and returns them in the review
receipt; strict legacy `id`/`note_id` input fields are refused. Candidate Markdown
must omit managed note identity.

`interpret_visual` transports one checked PNG through the explicitly selected
provider/model/effort, using the static visual behavior and no tools, history,
retry or fallback. `VisualImage` checks only its1MiB transport bound/signature;
Workflow qualifies complete PNG integrity, dimensions, provenance and freshness.
The prompt is bounded at64KiB and UTF-8 output at16KiB. Only a present final Stop
finish reason completes; missing/unknown/incomplete finish metadata refuses.
Description/uncertainty remain provisional until Workflow validates them and the
owner approves a separate exact Source annotation proposal.

## Authentication contract

- Share one `Arc<Auth>` for an application's explicit credential directory.
  The directory must be absolute, outside Git repositories, real (including
  ancestors), current-user-owned and exactly `0700`. Its parent must already
  exist. A missing directory is created; an unsafe existing directory is refused,
  not repaired. Unix filesystem checks are currently the supported platform.
- Opening `Auth` checks only the safe directory, so one provider's stale cache
  cannot block the other. Reusing known caches requires regular, single-link,
  current-user-owned `0600` files. Each reuse rechecks its provider's paths.
  Checked reads and permission
  changes use no-follow file descriptors and compare opened inode/device identity.
  Initial `0644` caches are refused; only files written during an authentication
  operation are tightened afterward.
- Only explicit `connect` enables the real Rig device flow. It always supplies
  `DeviceCodeHandler::new`, never Rig's code-printing fallback. The callback
  receives only the URL and code for the active, transient login surface.
  `LoginPrompt` and `ProviderClient` deliberately have no `Debug`.
- Authentication/refresh holds only the target provider's Tokio mutex.
  Authentication success, error and token cancellation all finalize permissions
  before releasing it; a tightening failure returns `UnsafeCredentials`, overriding
  success or an earlier error. Dropping an auth future also attempts synchronous
  tightening under its guard; prefer cancel-and-await so failures can be reported.
- `client` captures a concrete authenticated Rig client and a cloned `Selection`,
  with **no Auth borrow, mutex guard or retained authenticator**.
  `ProviderClient::selection()` exposes the frozen selection. `answer` consumes
  `ProviderClient.inner` / `auth::OwnedClient` within this crate: a boxed
  `openai::OpenAI` on the ChatGPT dialect, or `copilot::Copilot`.
- `disconnect` serializes cache deletion with authentication/refresh and removes
  only `chatgpt.json` / `chatgpt-name.json`, or `github-token` / `copilot.json` /
  `copilot-name.json`. In the checked safe folder, explicit Disconnect unlinks
  known own-UID regular files (including unsafe modes/hardlinks) and symlinks
  (including dangling links), without reading, following or chmod-repairing
  them. Outside targets are unchanged. Foreign-owned entries, directories and
  other non-file entries are refused; all target entries are checked before any
  deletion. Authenticators are operation-local, so there is no retained
  provider auth state to recreate a deleted cache.
  **The workflow's owned chat lane first fences new target-provider jobs, cancels/joins active
  login/refresh/discovery/turn jobs, finalizes the turn and drops owned clients.**
  `Auth` itself does not implement worker dispatch or streaming cancellation.

## Status, names and models

`status` never sends HTTP. `connected` means locally saved credential material,
including expired credentials; it does not promise upstream validity. Missing
identity is `name: None`; callers must display **“account name unavailable”**,
not infer an identity or switch accounts. ChatGPT's email is decoded from the
cached `id_token` **for display only**, not as authorization proof. On Copilot
Connect, an authenticated GitHub `/user` request uses the same Rig transport as
authentication. Provider-specific display metadata is stored only in `0600`
files in the credential directory, never SQL.

ChatGPT discovery authenticates with device flow disabled, then requests the
subscription `/models` catalog with an explicitly qualified Codex protocol
compatibility version (`0.161.0`) as `client_version`, independently of BRN's
crate release number. The backend gates model visibility on this compatibility
value: BRN package version `0.1.0` returned zero entries during the 2026-10-08
qualification; the qualified protocol value returned the current visible catalog,
including `gpt-6-luna`. This is not a caller-identity override, model selection or
entitlement guarantee. Changing the value requires catalog and pinned
Responses/tool/image contract qualification. It reuses pinned Rig 0.43's OpenAI Models wire encoding for the
URL and authorization, copying account and caller identity headers from the
same authenticated Rig config because the modality encoder omits them. Requests
use the existing Rig transport. A private decoder requires the Codex `models`
envelope and each entry's `slug`, known `visibility` and integer `priority`; the generic Rig decoder
expects the incompatible API `data`/`id` envelope. The catalog format follows
OpenAI's [endpoint implementation](https://github.com/openai/codex/blob/rust-v0.161.0/codex-rs/codex-api/src/endpoint/models.rs)
and [protocol](https://github.com/openai/codex/blob/afb436df8b70bb5bc57b86d9a3e829968988cd21/codex-rs/protocol/src/openai_models.rs).
All identifiers and duplicates are validated before returning any options.
Only `visibility: list` entries are shown, in ascending priority with ties
preserving server order. `supported_in_api` does not filter a subscription
catalog. An actual empty catalog returns an empty list; malformed responses and
failed requests safely fail without a substitute list.

Copilot discovery retains Rig's authenticated `list_models` route. Both providers
release the auth mutex before catalog network work and return only discovery
metadata, with every option `live_qualified: false`. Discovery does not establish
inference support or guarantee account entitlement, and never selects a model.

`Selection::validate()` checks only the exact identifier format for both
providers, preserving existing saved IDs and allowing explicit future IDs.
Workflow owns discovery membership policy and atomic persistence as one
`ai.selection` setting. There is no implicit selection.
Cancellation returns safe `Other`; the worker uses its cancellation token and
job identity to classify interruption. No operation automatically retries an
expired device code: `CodeExpired` supplies safe Retry copy for an explicit new
Connect action.

## Errors and offline verification

`AiError` contains only a kind and optional bounded unsigned reset delay (at most
365 days). Display uses static copy; no raw diagnostic, HTTP body, source chain,
token or device code is retained. Typed HTTP status takes priority: 401/403
reconnect, 429 rate limit; only allowlisted machine codes imply model refusal or
login expiry. Response-less transport failures map to Network. Cache safety,
storage and login-expiry failures remain distinct. Rig's fixed ChatGPT
string-wrapped auth failures are interpreted narrowly and immediately discarded.

`AiError::new(kind)` is public so workflow's `ReadTools` adapter can construct
safe typed failures without raw diagnostics. `Auth::credentials_dir()` exposes
only the configured checked folder location for workflow containment checks;
it never reads caches or authenticates.

Transport injection is crate-private and test-only (`Auth::with_http`); tests
exercise the **real pinned authenticators** with synthetic Rig HTTP transports.
There is no production fake-provider feature or dynamic provider registry.

## Read-only streamed answers

`answer(client, question, history, tools, cancel, emit)` consumes the owned
`ProviderClient` and uses its exact frozen model and provider. ChatGPT uses the
subscription Responses dialect; Copilot uses Rig's model-based routing to Chat
Completions or Responses. Authentication uses actual Rig configuration
`.connect(http).authenticate(...)`; ordinary client acquisition disables device
flow. No provider/model/account fallback or automatic transport/top-level retry is installed.
The pinned `rig-reqwest` shared client retains reqwest 0.13.5's default transport
policy: at most two extra sends for safe HTTP/2 protocol rejections (remote
GOAWAY `NO_ERROR` / `REFUSED_STREAM`). BRN does not retry HTTP statuses, model
refusal or a partially consumed answer. Synthetic transport tests bypass that
wire policy and cannot establish the number of real network sends.

Implement the synchronous `ReadTools: Send + Sync` seam in workflow and pass it
as `Arc<dyn ReadTools>`. Rig tool calls dispatch blocking reads via
`tokio::task::spawn_blocking`, with at most two concurrent calls. `search_notes`, `read_note`, `list_notes`, `read_action`, `list_actions` and `read_conflicts` are the
fixed read tools:

The three note tools accept a `scope` enum (`current`, `source`, `history`, `all`), defaulting
to Current when omitted. Serialized results label that scope alongside existing
fields. `ReadTools` scoped methods support this selection; old implementers
delegate omitted/Current calls and reject other scopes rather than substituting
current results. Workflow owns classification and fresh evidence checks. Scope
selects read-only evidence, never approval or mutation authority. The pinned
Copilot Responses strict schema makes every property required on the wire;
Rust omission compatibility remains supported and verified separately.

Each passage, read and list row requires `NoteFacts`: optional canonical managed
UUID string, complete saved-note SHA256 as the existing 32-byte JSON array,
independent `source`/`history` flags and tagged `conflicts: {status: "unknown"}`.
Workflow derives these from the same complete checked bytes before any text cap;
requested scope does not substitute for saved classification. Source and History
can overlap. A managed UUID is not a uniqueness or approval proof; classification
and provenance are evidence, never semantic truth. No eager conflict lookup occurs.

- Search queries are 1–512 **UTF-8 bytes**, limits are integers in 1–10 and
  returned hits cannot exceed the requested limit. `ToolSearch.keyword_only`
  reaches the model unchanged.
- Reads preserve the exact prefix, capped at `READ_NOTE_BYTES = 50_000`, cutting
  only at a UTF-8 boundary. `ToolNote.truncated` preserves an adapter's existing
  truncation flag or records this cap. No BOM/whitespace/CRLF normalization.
- List accepts optional folder/cursor and rejects adapter pages over 200 rows.
  Workflow owns vault/path/cursor validation, exclusion rules and fresh reads.
- AI Action reads return full approved current records, including immutable origins,
  plus a top-level `checked_ref` with UUID string, revision and lowercase SHA256
  of the complete canonical serialized `ActionRecord`. Replace proposals use that
  reference; Workflow checks the complete baseline. Owner `App::action` and
  `AppCommand::Action` records keep their existing shape.
  `read_action(id)` and `list_actions(state?, limit?, cursor?)` carry small protocol
  arguments only; workflow owns UUID/state/cursor checks and operational reads.
  Lists include all labeled states by default, limit1–20/default20 and opaque
  cursor≤256bytes. Full encoded JSON replies≤1MiB; oversized results are refused,
  never truncated. Existing note-only implementations safely refuse these methods.
- Conflict reads use `read_conflicts(path, scope?, limit?, cursor?)`. Path is 1–512
  UTF-8 bytes; scope defaults to Current and supports explicit Source/History/All.
  Limit is 1–100, default 10; cursor is an opaque optional string ≤8192 bytes, forwarded
  unchanged. Workflow owns saved path/identity matching, paging and fresh evidence
  observations. Whole encoded replies ≤1 MiB are returned intact or refused. Legacy
  read backends safely refuse. Ask instructions require looking up conflicts for
  relevant saved notes before claiming current facts, disclosing unresolved/stale
  evidence while allowing provisional recommendations with reasons and uncertainty. Incomplete pages/errors never mean no conflict.
  The same facts DTO adds `conflicts: {status: "known", open_count: N}`, bound to
  the existing exact managed `note_id` and full `SourceVersion`. The retained open
  count covers all pages, including findings with stale evidence; original fields,
  count, cursor and full inspections remain. Unknown is never zero, and even known
  zero cannot establish consistency, semantic truth or a winner.
- Argument schemas reject extra properties; Rust deserialization and validation
  also reject invalid arguments when the model ignores the schema. Safe failed
  tool results may continue the turn; they never authorize a retry or fallback.
  No comment, real Action write, Complete, approval, Save or account tool is exposed.

`answer_with_proposals` additionally registers fixed `propose_actions` through a
separate `ProposalTools` capability; ordinary read-only entry points and
Rewrite do not receive it. Input has only `title`, ordered `source_paths` and
1–20 typed `action_changes`, ≤8 MiB encoded. Create carries only `data`; Replace
carries `target: {id, version, sha256}` and `data`. No proposal/member identity or
full before-record is accepted from the model. Rust mints identities and loads
complete checked replacement baselines. Exact original input retries only within
the owning turn retain original proof and newer review; changed intent or another
turn creates separate review work.

Candidates require all14 semantic fields, including explicit nullable fields:
`title`, `description`, `state`, `owner`, `related_person`, `related_project`,
`sources`, `thread`, `due_on`, `follow_up_on`, `dependencies`, `parent`,
`follows_up`, `priority`. State is open/waiting/blocked; completed work uses a new
related Action and owner completion remains separate. Priority is null/low/normal/high.
Only Action relationships (`dependencies`, `parent`, `follows_up`) accept tagged
`{kind: "existing", id: UUID}` or `{kind: "member", index: 1..20}` references into
the same ordered proposal. Other UUID references remain strings. Protocol bounds
are checked here; Workflow owns UUID/date meaning, actual member length,
relationship validation, source capture, session inference and exact approval.
All input objects reject unknown fields, including legacy `id` and `before`.

Ordinary proposals capture their explicit source paths in caller order. For Inbox
analysis Workflow attaches the selected Source path and note identity automatically;
`source_paths` carry additional evidence. The tool schema is derived once with
Schemars from these Serde types (`ActionProposalArgs`, `ActionCandidate` and its
parts), then inlined and adapted; a frozen copy of the earlier hand-written schema
checks equivalence in tests. Its length bounds are hints; `validate` enforces UTF-8
byte limits. The wire uses closed `anyOf` variants
and typed discriminant enums from the
[official supported schema subset](https://developers.openai.com/api/docs/guides/structured-outputs#supported-schemas).
Synthetic real Rig routes qualify typed dispatch and strict refusal; live provider
acceptance remains unqualified. The tool dispatches on spawn_blocking with the
same shared limits, returning only a whole ≤1 MiB receipt, never candidate/comment
bodies. It creates review work only; human exact approval remains separate.

The same `ProposalTools` capability may explicitly opt in to `propose_knowledge`
for a workflow-owned Ask job bound to a selected approved Inbox Source. Ordinary
backends default to disabled and refuse the callback; read-only Ask and Rewrite
never register this tool. Its closed, all-required input is proposal UUID/title,
relative destination path, stable note UUID, complete candidate Markdown and
exact saved body quotations, ordered additional `source_paths` (0–63 entries,
each1–512 UTF-8 bytes), and nullable `supersedes` (one1–512byte Current path).
The outward schema requires all eight fields; omitted legacy `source_paths`
deserializes as empty and omitted `supersedes` as None. A predecessor consumes
one proof slot, limiting additional paths to62. UUID strings are1–64bytes, title/path1–512bytes,
text1byte–1MiB, quotes1–32 with each quotation1–16KiB of exact UTF-8 bytes; complete
encoded input≤8MiB and whole receipts≤1MiB. These are protocol bounds only.
Workflow checks UUID/path/source/current identity and citation rules, adds exact
saved citations, and creates one independent Current knowledge review draft.
Each quote supplies `quote` and optional nullable `occurrence`, a 1-based match in
the saved body. Without occurrence, wording must match uniquely. Workflow resolves
exact ranges and returns typed not-found, ambiguous or invalid-occurrence refusals;
the adapter accepts integer occurrence values for that domain check. Negative and
fractional values and legacy `start_byte`/`end_byte` fields refuse deserialization.
Copilot Responses' Rig schema normalization requires nullable `occurrence` on the
wire; ChatGPT and Copilot completion keep it optional. Omitted and null become None.
Knowledge and conflict tools return the three quotation refusals as bounded
`{"error":{"kind":"quote_not_found|quote_ambiguous|quote_occurrence_invalid","message":"fixed safe text"}}`
feedback. Other errors keep Rig's generic failure behavior; no raw diagnostics are exposed.
With supersedes, workflow captures the predecessor automatically as second proof,
adds its Previous version link and a protected History Replace to that same
proposal. Source/History predecessors refuse; do not repeat the predecessor in
additional paths. Unresolved authority should be reported as conflict, not guessed. This tool never approves or writes knowledge. Like Action proposals, its
callback uses `spawn_blocking` and shares the existing tool-round budget.
The selected Inbox Source is automatically the mandatory first proof and must
not be repeated among additional paths. Stable `brn://note/UUID` relationships
require exact named target evidence. Read tools default to Current; explicitly
named extra Source/History paths are evidence, never truth or deletion approval.
Workflow captures and qualifies the complete proofs; human exact approval stays
separate.

Inbox's same `knowledge_enabled()` opt-in also registers `report_conflict`.
Its strict five-field input is nonblank `title`, nonblank `summary`,
`source_quote`, `other_path`, and `other_quote`. Both quote objects require exact
`quote` wording with the same optional occurrence contract. Protocol bounds are
title ≤512 bytes, summary ≤16 KiB, path 1–512 bytes and quote 1–16 KiB.
Complete encoded input ≤512 KiB; whole receipts ≤1 MiB, never clipped.
Workflow assigns the finding UUID from its analysis and exact candidate intent,
returns it in the receipt and owns path/body/proof validation. Identical intent
retries return the same finding; changed intent creates a separate draft.
Caller-supplied finding IDs are rejected. Workflow
persists only a tentative unresolved finding with two opposing saved
quotations. Its summary may contain a provisional preferred resolution, reasons,
alternatives and uncertainty. The adapter grants no knowledge effects, real Actions
or deletion authority; exact proposals and human approval still govern those.
Ordinary Ask and Rewrite never receive this report capability; legacy proposal
backends reject it. Dispatch uses the existing blocking lane and shared round
budget. Synthetic real Rig routes qualify availability, strict schema/arguments,
exact dispatch, defaults/scopes/cursors/pages, errors, complete byte limits and
unresolved/stale-answer instructions; live model compliance remains unqualified.

Invalid scope/type/extra arguments invoke no underlying read. Rig may return its
parse diagnostic transiently to the model that generated the invalid argument;
local progress still exposes only allowlisted tool names and typed BRN errors.
BRN's repository-owned exact Rig0.43.0 [safety patch](../../vendor/README.md)
removes one unconditional dependency stderr print before invalid-tool hooks.
Unknown tools and malformed proposal/mutation tools retain InvalidToolUse,
with no dispatch or correction. Strict Rewrite and standalone visual interpretation
remain unchanged. A child-process regression checks private arguments, correction
feedback and prior partial text never reach stderr. This does not authorize verbose upstream tracing or establish a
general provider logging guarantee.

History is limited to the last 20 earlier `HistoryPair` values, converted to
text-only user/assistant messages. Empty assistant text is omitted on the wire
while its question is retained. Earlier tools, results, reasoning and provider
response IDs are not restored. A run-owned model-finished hook counts each
tool-containing response once, including parallel calls. Compatibility entrypoints
allow eight rounds and a ninth final answer. Investigation entrypoint
`answer_with_proposals_and_images_with_limit` accepts validated1..32 rounds;
Rig max_turns is rounds+1 and an excess tool round stops **before dispatch**.
Standard Ask and Inbox investigations, including proposal-capable Ask, permit
one correction of syntactically non-JSON arguments for an allowlisted read tool
that is both available and allowed. Pinned Rig's `InvalidToolCallAction::retry`
abandons the entire rejected response before dispatch, including valid read or
proposal peers, and supplies static JSON/schema feedback without raw arguments or
parser diagnostics. The first surfaced invalid call controls this decision; peers
later in the abandoned stream are drained without parsing or dispatch. The rejected response consumes one existing model-call slot
and no tool round; it adds no allowance. A second malformed call refuses. Typed
schema/domain/backend failures retain normal tool-result continuation. Rewrite
and standalone visual interpretation keep zero invalid-call retries.

The final model-call slot must be tool-free, even if a malformed response left
unused tool rounds. Investigation `BudgetProgress.model_turns` retains its field
name but counts **model requests admitted** at Rig's completion-call boundary,
after the cancellation guard; the one-based index includes malformed responses
and refused final responses. Progress repeats the same model count when a valid
tool round is admitted; parallel calls consume one round. These counters are not
HTTP sends, provider reasoning turns, token use or spend. A run-owned atomic flag
classifies `ToolLimitReached`, independently of Rig's stop-reason wording.

For `answer`, `AiEvent::Text` appends/emits each fragment exactly once;
`AiEvent::ToolStarted` contains only an allowlisted tool name, never its
arguments/results. `AiAnswer.text` is exactly the emitted text, including partial
text on `Failed` or `Interrupted`; final response output is not appended again.
The first stream error stops collection; EOF without a final response fails.
Stop drops the local stream, not a guarantee of upstream cancellation or zero
billing. Already consumed final completion wins over a later Stop. Blocking
reads already started may finish after local cancellation. The workflow's owned
turn lease waits for every retained blocking reader before terminal persistence,
model/tool replacement or owner release.

Internal `provider_formats_tests.rs` uses ordered unary auth/identity replies
and a private queue of **distinct responses per streaming request**, not one
chunk queue reused across completions. Every destination, including absolute
auth/GitHub URLs, is intercepted; unexpected requests fail and scripts must be
fully consumed. Tests run real authenticated production clients/agents for all
three wire routes, continuation/history/exact models, error mapping, caps and
eight-versus-nine round accounting. These are offline format checks, not live
account qualification.

## Narrow capability probe

The optional `capability-spike` feature exposes `capability_probe` and the
`provider-capabilities` example for [roadmap Stage 3](../../docs/work/completed/provider-capabilities/plan.md).
Ordinary app builds do not expose this entry point. It consumes an explicitly
selected authenticated client; it never chooses a provider/model, signs in,
retries an application request or writes knowledge. Low/high effort requests
allow one synthetic `read_note` call and at most two completions. Image input is
a fixed 16×16 PNG. Web probes use the Responses hosted web tool and retain
bounded HTTP(S) citations plus observed completed web calls. Copilot Chat web
is refused before streaming because the pinned adapter has no native web seam.

Reports preserve bounded partial text, safe typed failure/interruption and
deduplicated source URLs/titles. Failures project only fixed error class, numeric
HTTP status and allowlisted rejection categories, without raw metadata/errors.
The demonstrated `unsupported_api_for_model` rejection maps to `ModelRefused`
in normal chat; the selected route and model stay unchanged. The
example requires `--live`, an exact model and an absolute task credential path;
it cancels and awaits on SIGINT or after 120 seconds. Its account must already be connected.
Running it requires the owner's separate live-account authorization. Building
and testing it does not establish endpoint capability or enable these inputs
in ordinary chat.

```sh
# Offline preparation only.
cargo build -p brn-ai --example provider-capabilities --features capability-spike --locked --offline
cargo test -p brn-ai --features capability-spike --locked --offline
# After separate authorization and explicit connection in a fresh task folder:
target/debug/examples/provider-capabilities --live chatgpt /private/tmp/brn-provider-qualification/credentials gpt-5.5 low
```

```sh
# Run from the workspace root; no TMPDIR override is needed.
cargo test -p brn-ai --lib --locked --offline
cargo test -p brn-ai --locked --offline
cargo clippy -p brn-ai --all-targets --locked --offline -- -D warnings
cargo fmt --all -- --check
cargo tree --workspace --all-features --locked --offline -i ort
cargo tree --workspace --all-features --locked --offline -i libsqlite3-sys
```

The facade enables only `agent`, `derive`, `reqwest`; TLS is enabled directly on
`rig-core` and `rig-reqwest`. Never add facade TLS/fastembed/SQLite features.
This preserves the [spike's pinned dependency graph](../../experiments/rig-spike/FINDINGS.md).
Offline tests do not qualify live authentication, native UI, server-side
cancellation or billing behavior.

## P2 private intake evidence

The existing Rig proposal runtime also accepts a collection of exact retained PNG
assets with explicit source/occurrence labels for private EML/DOCX investigation.
JPEGs remain reviewable extraction assets but are not supplied through this PNG
transport. Workflow binds proposals to the pending Source and validates quoted
source-node ranges; the model cannot approve or create a saved Source version.
The existing 50,000-byte captured Source bound remains; larger semantic-context
work is outside P2. Synthetic tool tests verify authority and lifecycle, not live
model usefulness.

For KnowledgeAndActions intake investigations, `report_conflict` may retain exact
opposing saved body quotations only when the bound Source is already Applied.
Workflow qualifies its historical installed proof and current prerequisite;
pending intake conflicts stay in the answer. The existing Rig tool/schema and
tentative Finding lifecycle are reused, without model approval authority or new
private-intake cleanup support. Previously captured questions remain unchanged.

## Hash-bound long evidence reads

`read_note` retains its50,000-byte UTF-8 prefix behavior. `read_note_range`
reads an exact zero-based half-open UTF-8 byte interval of up to50,000 bytes from
a supported saved note. Copy `expected_sha256` from fresh read/search/list facts;
never silently refresh that proof. Empty intervals return `total_bytes` for
length discovery. Offsets include BOM/frontmatter/CRLF; invalid boundaries,
reversed/oversized/out-of-file spans and stale whole-file hashes refuse without
clipping. A changed byte outside the interval still makes the proof stale.
Scope defaults Current; explicit Source/History/All preserve their existing
eligibility and complete-note facts. The adapter refuses inconsistent backend
results and legacy backends refuse unsupported range reads. The existing eight
tool-round budget, frozen selection and joined blocking-read lifecycle apply.

Retained intake guidance distinguishes pending/Applied Sources, inventories
material details across processed headers/footers/tables, and separates exact
quotations from rendered/paraphrased text. A rejected quotation may be corrected
within the existing budget; this grants no fuzzy matching, approval or semantic
completeness authority. Shared image placements remain distinct from image bytes.
Live usefulness and interactive acceptance are separate qualification gates.


Shared Action guidance explicitly separates review preparation from execution
authority. A useful supported clarification/response/owner-decision draft may have
an unknown owner (`null`) and unresolved conditions; it implies no assignment,
acceptance, release, spending permission or completion. Informational evidence
with no useful follow-up may produce no Action draft. No count or semantic answer
is forced. Real-Rig synthetic route tests qualify delivery and the nullable-owner/
abstention contract; actual model usefulness needs separate bounded live evidence.
