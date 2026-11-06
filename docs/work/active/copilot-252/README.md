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

## Limits

Device sign-in, account switching in the native UI and native observation remain owner or
native-pending; this record does not claim them. Not yet qualified live on Copilot: a clean
installed Mac, reconnect after restart, consolidation, referenced writing with
Discard/Undo/stale refusal, quota or policy denial, and an account without Luna. The
capability boundary is what BRN requests and verifies in each reply (requested tool set,
echoed tools, every output item); it cannot observe provider internals beyond that.

A single model turn has no wall-clock or byte bound while bytes keep arriving: idle
detection does not fire and the 128 KiB answer cap excludes tool-call arguments. Stop
still ends such a run. The strict-schema trigger is fixed; a turn bound is follow-up work
(#255).
