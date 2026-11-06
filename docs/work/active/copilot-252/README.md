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
  account also counts as a change (conservative).
- **No fallback.** A Copilot selection never reads the Codex auth file; a missing Copilot
  sign-in refuses with zero requests. Settings provider strings map through
  `Provider::from_setting` only (`codex`/`chatgpt`, `copilot`).
- **Native web search.** `copilot_search.rs` sends one Rig Responses request on the selected
  account and chat model: `tools:[{"type":"web_search"}]`, `tool_choice:"required"`, bounded
  `max_tool_calls`, the query as the single input item (no notes, documents, history or BRN
  instructions) and source inclusion. It refuses any function/MCP/other tool call, echoed
  tool widening, missing completed search or uncited answer. Sources come from
  `url_citation` annotations and `web_search_call.action.sources`; non-http(s) URLs are
  dropped. Cancellation, deadlines and query bounds reuse the existing search contract, and
  `Allow web access`, Off/no-dispatch and run-scoped result fencing are unchanged in the
  threads app. Models that reject hosted search (`unsupported_api_for_model`, e.g. Claude)
  report `CopilotModelCannotSearch`.
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
all pass (brn-ai 593 passed, 1 ignored); workspace Clippy with `-D warnings`, with and
without `capability-spike`, and `cargo fmt --check` are clean. Document tests need both
`BRN_XBERG_HELPER` and `BRN_XBERG_HELPER_BRN`. Council and hosted CI: pending on the PR.

## Limits

Device sign-in, account switching in the native UI and native observation remain owner or
native-pending; this record does not claim them.

A single model turn has no wall-clock or byte bound while bytes keep arriving: idle
detection does not fire and the 128 KiB answer cap excludes tool-call arguments. Stop
still ends such a run. The strict-schema trigger is fixed; a turn bound is follow-up work
(#255).
