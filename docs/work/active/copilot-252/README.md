# Copilot chat, native web search and Luna refinement (#252)

Owner: Agent A (runtime). Branch `work/copilot-252`, stacked on the Rig 0.46 runtime (#259).
Issue: https://github.com/ewq100/brn-rust/issues/252

Goal: an alpha tester uses BRN with their own GitHub Copilot subscription, with no Codex
credentials, API key, development checkout or terminal credential setup.

## Design

- **Sign-in and transport.** Existing device sign-in (`Auth`, Rig Copilot dialect) and the
  existing Rig transport. No new dependency, no Copilot CLI or SDK subprocess, so packaging
  has nothing extra to bundle.
- **Account pin.** `AccountPin` records the Copilot bootstrap fingerprint when an operation
  starts: chat run preparation, helper choice, chat web searcher preparation and document
  job start. Before building a client and again after the session exchange, the pin is
  compared with the current sign-in; a change refuses with `account_changed`
  (ReconnectNeeded) before any model request. A Copilot client's session token is fixed at
  creation, so a started request cannot move to another account. Re-signing in to the same
  account also counts as a change (conservative). When no account could be read at start
  (unreadable credentials), the operation refuses with ReconnectNeeded and never adopts a
  sign-in made later.
- **No fallback.** A Copilot selection never reads the Codex auth file; a missing Copilot
  sign-in refuses with zero requests. Settings provider strings map through
  `Provider::from_setting` only (`codex`/`chatgpt`, `copilot`).
- **Native web search.** `copilot_search.rs` sends one Rig Responses request on the selected
  account and chat model: `tools:[{"type":"web_search"}]`, `tool_choice:"required"`,
  `max_tool_calls` 3 (also enforced locally on the returned search calls), the query as the
  single input item (no notes, documents or history; only a fixed one-sentence search
  preamble is sent as instructions) and source inclusion. It refuses any function/MCP/other
  tool call, echoed tool widening, missing completed search or uncited answer. Sources come from
  `url_citation` annotations and `web_search_call.action.sources`; non-http(s) URLs are
  dropped. Every response body is capped at 8 MiB while it is received, before JSON
  decoding, so large reasoning or source metadata cannot exhaust memory. Cancellation, deadlines and query bounds reuse the existing search contract, and
  `Allow web access`, Off/no-dispatch and run-scoped result fencing are unchanged in the
  threads app. Models that reject hosted search (`unsupported_api_for_model`, e.g. Claude)
  report `CopilotModelCannotSearch`; other model refusals (e.g. `model_not_found`) report
  `CopilotModelUnavailable`. HTTP 401 asks to connect Copilot; HTTP 403 from a signed-in
  account reports `CopilotSearchDenied` (subscription, permission or policy) instead.
  A Copilot credentials folder without a sign-in shows web access as Unavailable.
- **Luna refinement.** `ConversionAccount` is captured at document job start from the
  selected provider. Copilot refinement uses `gpt-6-luna`, high effort, on the Copilot
  Responses route; ChatGPT keeps its existing route. Off never opens credentials.

## Paths audited for the selected provider/account

| Path | Captured at | Copilot account |
|---|---|---|
| Chat run and chat summary (shared client) | `prepare_run` | pinned |
| Helpers (`threads_research`) | `HelperChoice::from_settings` | pinned |
| Web access (`threads-app` searcher) | run preparation | pinned |
| Document refinement (desktop, inbox, CLI) | job start | pinned |
| Model discovery / sign-in UI | user action | no model request |
| `capability_probe`, provider fixtures | diagnostics/tests | not in alpha journeys |

## Live qualification

Probe on a private copy of the owner's Copilot business sign-in (credentials never stored in
evidence). `gpt-6-luna` is served on `/responses` with efforts none through max. Hosted
`web_search` returned `web_search_call` sources and `url_citation` annotations for
`gpt-6-luna`, `gpt-6.1-sol` and `grok-4.7`; Claude models returned 400
`unsupported_api_for_model`.

Integrated-branch run (fresh data folder, `--provider Copilot`, no Codex credential file):

| Journey | Model | Result |
|---|---|---|
| Multi-turn chat | `gpt-6-luna` | answered; same-model Chat → Responses retry only |
| Agent web search, web access on | `gpt-6-luna` | `web_search` succeeded with 18 sources; the answer and persisted message cite real URLs |
| BRN read tools | `gpt-6-luna` | `read_skill`, `search_notes` across turns |
| Write proposal and Apply | `gpt-6-luna`, `gpt-6.1-sol` | `prepare_changes` → `apply_changes` Applied (after the strict-tools fix below) |
| Document intake, refinement Off | n/a | Applied, settings `ai:false` |
| Document intake, refinement On | `gpt-6-luna` (high) | Applied in 4.5 s, settings `ai:true`, structure restored |
| Web search on a model without hosted search | `claude-sonnet-5.5` | explicit "can't search the web" failure; no model switch |
| CLI cancellation (SIGTERM) mid-stream | `gpt-6.1-sol` | run ended Interrupted; nothing written |
| Agent web search after the Council body cap | `gpt-6-luna` | `web_search` succeeded with 13 sources through the byte-capped transport |

Discovered Claude identifiers on this account: `claude-haiku-5.5`, `claude-opus-4.8`,
`claude-opus-5`, `claude-opus-5.5`, `claude-sonnet-5`, `claude-sonnet-5.5`.

### Defect found: Copilot write proposals always failed

Rig's Copilot dialect turns on strict tool schemas for the Responses route (0.45 and
0.46 alike). Strict sanitizing rewrote `prepare_changes` record `data` to a closed empty
object, so every proposal was rejected as an invalid payload. With `gpt-6.1-sol` the
constrained stream also ran away (8+ minutes, 34 MB) until stopped. `auth::copilot_completion`
now sends non-strict tools on both Copilot routes; BRN already validates every argument.
`copilot_tools_keep_open_object_schemas_on_every_route` fails without the fix, and the
writing contract test now requires Copilot to receive exactly the Codex schema.

## Evidence

Local, stacked on the #259 runtime: `cargo test -p brn-ai -p brn-threads-app -p brn -p brn-desktop`
all pass (brn-ai 597 passed, 1 ignored after Council fixes); workspace Clippy with
`-D warnings`, with and without `capability-spike`, and `cargo fmt --check` are clean.
Document tests need both `BRN_XBERG_HELPER` and `BRN_XBERG_HELPER_BRN`. Live runs used debug
`brn` builds of this branch: the table above through `b42331e0`, the body-cap search row
after the Council fixes. Council and hosted CI are recorded on PR #261.

Council round 1 fixes: a missing start account now refuses instead of capturing later
(`copilot_run_without_a_start_account_never_adopts_a_later_sign_in`, red without the fix);
search bodies are bounded while received (`an_oversized_reply_is_refused_while_it_is_received`);
the search tests that need a signed-in credential folder run on Unix only, because Windows
credential storage refuses before any effect; a credentials folder without a sign-in shows web
access Unavailable; 403 policy denial and unavailable models get their own messages
(`policy_denial_and_unavailable_models_are_not_reported_as_sign_in`); more search calls than
requested are refused (`more_searches_than_requested_are_refused`).

Deferred Council nits, none of which can serve another account: the chat pin and the search
pin are captured separately at run start, so a sign-in between them only makes search refuse
as account-changed; non-GPT models still show web access Allowed and each search reports
that the model cannot search; the opt-in capability probe and provider fixtures build
Copilot models directly rather than through `copilot_completion`; provider-string parsing
and the token-digest account pin are candidates for consolidation (#255).

## Release-hardening follow-up (`work/response-limits-252`)

Owner: Agent A. Scope: the response bound below, an honest notice for a Copilot chat
HTTP 403 and the remaining #252 journeys. This is release hardening for #252/#253, not #255
development-speed work; #255 keeps only the provider-module consolidation nits above.

**Response bound.** A model response that kept sending bytes previously had no wall-clock
or byte bound, and tool-call arguments escaped the 128 KiB answer cap.
`crates/brn-ai/src/response_limit.rs` gives each model HTTP response of a run 20 minutes
from send and 64 MiB of received wire bytes (text, reasoning, tool-call arguments and
framing). The bound is enforced while the stream is read, before Rig buffers the bytes, on
the run transport shared by chat, summary and helpers, for Codex and Copilot alike. A trip
ends the body with a transport error, so Rig fails the turn before any of its tool calls
dispatch. Whole-body replies (account and token exchanges) are held to the duration only;
model responses stream. The run fails as `TimeLimitReached` or `Other` with origin `response_limit`,
non-retryable, with "A model response reached BRN's time/size limit. Continue starts
fresh." The guard then refuses every later request of the run, so a request the provider may
have processed is never resent, and accepted progress stays. Rig's 300 s idle bound still ends
a silent stream; Stop stays immediate. Headroom: about 32k output tokens at a slow 40
tokens/s take under 14 minutes; a full 128k-token response is about 26–32 MB on the wire.
Document conversion/refinement keep their own total timeout and 1 MiB text cap, and search
keeps its 8 MiB caps. See [threads behavior](../../../architecture/threads-behavior.md).

**Copilot chat 403.** A signed-in account refused by subscription, permission or policy
read "The provider account needs reconnecting", which reconnecting alone does not fix. Any
provider HTTP 403 now reads "The provider refused this account's request (subscription,
permission or policy). Check the account's access or reconnect it, then Continue." (a
ChatGPT token-refresh 403 can still need a reconnect). The kind stays `ReconnectNeeded` for
the CLI contract; missing credentials and HTTP 401 keep the reconnect notice.

**Deterministic checks** (short synthetic limits; `threads_response_limit_tests.rs`,
`threads_copilot_denial_tests.rs`, `document_refine_tests.rs`, `response_limit.rs`):
a Copilot keep-alive stream reaches the time limit with one request and no Responses retry;
a Codex stream without end fails after accepted progress and sends nothing more; oversized
tool arguments fail on size with no `ToolStarted`; normal completion under short limits;
Stop during an active stream ends Interrupted promptly; unit checks for the wire cap,
the deadline, unary bodies and refusal after a trip. Copilot simulated denials: quota 429 →
usage limit, no other model or ChatGPT route; policy 403 → the access notice above (red before
the fix); an account without `gpt-6-luna` → model refused in chat and in refinement, with no
substitute model. `cargo test -p brn-ai --lib` 610 passed, 1 ignored; `brn-threads-app`,
`brn`, `brn-desktop` all pass; workspace Clippy `-D warnings` with and without
`capability-spike` and `cargo fmt --check` clean.

**Live journeys** (debug `brn` of `b6840357`, fresh test workspace, private copy of the owner's
Copilot business sign-in, `gpt-6-luna` high; credential copy and data deleted after):

| Journey | Result |
|---|---|
| Knowledge consolidation | Two overlapping, contradictory synthetic notes → one review candidate keeping every quantity, the superseded 9 m/s and timber-mat figures as history, the P7/P8 exception change and both source links; Apply (2 writes) then Undo restored both notes |
| Referenced writing, `--edit-note` grant | Edit referencing the other note applied under the one-invocation grant; Undo restored the note |
| Referenced writing, Discard | Review candidate dismissed; note unchanged, no candidate left |
| Stale-edit refusal | Candidate prepared, owner saved an edit, Apply → Stale (exit 3); owner text kept, candidate text absent |
| Reconnect and restart | Cached Copilot session deleted; a new process re-exchanged the session and continued the earlier chat with correct context |
| Intake with refinement On, web search | Applied; `web_search` ×2 cited a real release page |
| Codex recheck (owner's Codex sign-in, `gpt-6.1-sol` low) | `search_notes`/`read_note` answer completed through the guard |

Simulated only (this business account has Luna and no quota/policy limits to trigger):
quota and policy denial, an account without Luna, and account switching (account-pin tests).

## Limits

Device sign-in, account switching in the native UI and native observation remain owner or
native-pending; this record does not claim them. Not yet observed: a clean installed Mac (on
B's #253 candidate), the desktop journeys after an update, and live quota, policy or
no-Luna accounts (simulated only). The capability boundary is what BRN requests and verifies
in each reply (requested tool set, echoed tools, every output item); it cannot observe
provider internals beyond that. The response bound is per model response with run-wide
refusal after a trip. A run's total time is bounded by its tool rounds, helper and
compaction turns at 20 minutes per response, or by a model-call limit when the host sets one.
