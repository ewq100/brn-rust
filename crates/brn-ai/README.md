# brn-ai

Thin, fixed ChatGPT/Copilot subscription authentication and streamed
chat over Rig **0.43.0**. Contains account/selection DTOs, safe errors, checked
credential storage, owned clients, six read tools and one separate review-proposal capability. It does not contain
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

Static Ask, Action-review, Inbox-knowledge and Rewrite instructions/capability
selection live behind the private typed `behavior` boundary. Individual tool
descriptions remain with their implementations; workflow supplies captured task
context through its private typed boundary and retains deterministic authority.

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
subscription `/models` catalog with BRN's actual package version as
`client_version`. It reuses pinned Rig 0.43's OpenAI Models wire encoding for the
URL and authorization, copying account and caller identity headers from the
same authenticated Rig config because the modality encoder omits them. Requests
use the existing Rig transport. A private decoder requires the Codex `models`
envelope and each entry's `slug`, known `visibility` and integer `priority`; the generic Rig decoder
expects the incompatible API `data`/`id` envelope. The catalog format follows
OpenAI's [endpoint implementation](https://github.com/openai/codex/blob/afb436df8b70bb5bc57b86d9a3e829968988cd21/codex-rs/codex-api/src/endpoint/models.rs)
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
flow. No provider/model/account fallback or application retry is installed.
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

- Search queries are 1–512 **UTF-8 bytes**, limits are integers in 1–10 and
  returned hits cannot exceed the requested limit. `ToolSearch.keyword_only`
  reaches the model unchanged.
- Reads preserve the exact prefix, capped at `READ_NOTE_BYTES = 50_000`, cutting
  only at a UTF-8 boundary. `ToolNote.truncated` preserves an adapter's existing
  truncation flag or records this cap. No BOM/whitespace/CRLF normalization.
- List accepts optional folder/cursor and rejects adapter pages over 200 rows.
  Workflow owns vault/path/cursor validation, exclusion rules and fresh reads.
- Action reads return full approved current records, including immutable origins.
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
  evidence and choosing no winner. Incomplete pages/errors never mean no conflict.
- Argument schemas reject extra properties; Rust deserialization and validation
  also reject invalid arguments when the model ignores the schema. Safe failed
  tool results may continue the turn; they never authorize a retry or fallback.
  No comment, real Action write, Complete, approval, Save or account tool is exposed.

`answer_with_proposals` additionally registers fixed `propose_actions` through a
separate `ProposalTools` capability; ordinary read-only entry points and
Rewrite do not receive it. Input has only proposal UUID/title, ordered source paths
and1–20 whole Create/Replace members, ≤8MiB encoded. Candidates require all14 fields
and nulls; Replace requires the complete before-record. Workflow owns UUIDs,
references, source capture, session inference, exact creation replay and approval.
The wire uses closed `anyOf` variants and typed discriminant enums from the
[official supported schema subset](https://developers.openai.com/api/docs/guides/structured-outputs#supported-schemas);
workflow validates uniqueness and byte limits. This does not qualify live provider
acceptance. The tool dispatches on spawn_blocking with the same shared limits, returning only
a whole ≤1MiB receipt, never candidate/comment bodies. It creates review work only.

The same `ProposalTools` capability may explicitly opt in to `propose_knowledge`
for a workflow-owned Ask job bound to a selected approved Inbox Source. Ordinary
backends default to disabled and refuse the callback; read-only Ask and Rewrite
never register this tool. Its closed, all-required input is proposal UUID/title,
relative destination path, stable note UUID, complete candidate Markdown and
exact source byte ranges, ordered additional `source_paths` (0–63 entries,
each1–512 UTF-8 bytes), and nullable `supersedes` (one1–512byte Current path).
The outward schema requires all eight fields; omitted legacy `source_paths`
deserializes as empty and omitted `supersedes` as None. A predecessor consumes
one proof slot, limiting additional paths to62. UUID strings are1–64bytes, title/path1–512bytes,
text1byte–1MiB, quotes1–32 with start<end≤50,000 and each span≤16KiB; complete
encoded input≤8MiB and whole receipts≤1MiB. These are protocol bounds only.
Workflow checks UUID/path/source/current identity and citation rules, adds exact
saved citations, and creates one independent Current knowledge review draft.
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
Its strict six-field input is `id`, nonblank `title`, nonblank `summary`,
`source_quote`, `other_path`, and `other_quote`. Both quote objects require exact
`start_byte`, `end_byte`, and `quote` wording. Protocol bounds are id 1–64 bytes,
title ≤512 bytes, summary ≤16 KiB, path 1–512 bytes, quote 1–16 KiB with start<end≤1 MiB
and exact quoted byte length matching the span. Complete encoded input ≤512 KiB;
whole receipts ≤1 MiB, never clipped. Workflow owns UUID/path/body/proof validation
and persists only a tentative unresolved finding with two opposing saved
quotations. The adapter gives no winner, knowledge effects, real Actions or
deletion authority; exact proposals still govern knowledge and real Actions.
Ordinary Ask and Rewrite never receive this report capability; legacy proposal
backends reject it. Dispatch uses the existing blocking lane and shared round
budget. Synthetic real Rig routes qualify availability, strict schema/arguments,
exact dispatch, defaults/scopes/cursors/pages, errors, complete byte limits and
unresolved/stale-answer instructions; live model compliance remains unqualified.

Invalid scope/type/extra arguments invoke no underlying read. Rig may return its
parse diagnostic transiently to the model that generated the invalid argument;
local progress still exposes only allowlisted tool names and typed BRN errors.

History is limited to the last 20 earlier `HistoryPair` values, converted to
text-only user/assistant messages. Empty assistant text is omitted on the wire
while its question is retained. Earlier tools, results, reasoning and provider
response IDs are not restored. A run-owned model-finished hook counts each
tool-containing response once, including parallel calls. Exactly eight such
rounds are allowed; a ninth tool round is stopped **before dispatch**.
`max_turns(9)` leaves room for eight tool rounds plus a ninth final answer;
invalid-tool retries are explicitly zero. A run-owned atomic flag classifies
`ToolLimitReached`, independently of Rig's stop-reason wording.

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
