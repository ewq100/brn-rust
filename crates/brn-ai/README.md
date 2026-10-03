# brn-ai

Thin, fixed ChatGPT/Copilot subscription authentication and streamed read-only
chat over Rig **0.43.0**. Contains account/selection DTOs, safe errors, checked
credential storage, owned clients and three read tools. It does not contain
workers, SQL, selection persistence or frontend state.

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

ChatGPT models are a maintained subscription list: currently only `gpt-5.5`,
`live_qualified: false`. Its broken upstream `list_models` is never called.
Copilot discovery authenticates with device flow disabled, releases the auth
mutex, then calls Rig's real `list_models`. Discovery failures are returned
without a substitute list/model/provider; all returned options remain locally
unqualified until separately authorized live acceptance.

`Selection::validate()` rejects invalid identifiers and ChatGPT models outside
the maintained list. Copilot discovery membership and atomic persistence as one
`ai.selection` setting belong to workflow. There is no implicit selection.
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
`tokio::task::spawn_blocking`, with at most two concurrent calls. Only
`search_notes`, `read_note` and `list_notes` are registered:

- Search queries are 1–512 **UTF-8 bytes**, limits are integers in 1–10 and
  returned hits cannot exceed the requested limit. `ToolSearch.keyword_only`
  reaches the model unchanged.
- Reads preserve the exact prefix, capped at `READ_NOTE_BYTES = 50_000`, cutting
  only at a UTF-8 boundary. `ToolNote.truncated` preserves an adapter's existing
  truncation flag or records this cap. No BOM/whitespace/CRLF normalization.
- List accepts optional folder/cursor and rejects adapter pages over 200 rows.
  Workflow owns vault/path/cursor validation, exclusion rules and fresh reads.
- Argument schemas reject extra properties; Rust deserialization and validation
  also reject invalid arguments when the model ignores the schema. Safe failed
  tool results may continue the turn; they never authorize a retry or fallback.
  No comment/proposal/write tools are exposed.

History is limited to the last 20 earlier `HistoryPair` values, converted to
text-only user/assistant messages. Empty assistant text is omitted on the wire
while its question is retained. Earlier tools, results, reasoning and provider
response IDs are not restored. A run-owned model-finished hook counts each
tool-containing response once, including parallel calls. Exactly eight such
rounds are allowed; a ninth tool round is stopped **before dispatch**.
`max_turns(9)` leaves room for eight tool rounds plus a ninth final answer;
invalid-tool retries are explicitly zero. A run-owned atomic flag classifies
`ToolLimitReached`, independently of Rig's stop-reason wording.

`AiEvent::Text` appends/emits each fragment exactly once;
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
