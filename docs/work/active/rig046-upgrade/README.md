# Rig 0.46 upgrade and bounded reuse (#253)

Runtime PR for the first limited Mac alpha. Branch `work/rig-046-253`, based on
`origin/main` `2ebcf4c76f69c3c6ab9eb0c4efeb84b6718ebc8a` (UI #248 and chat #250
merged). Copilot chat, web search and Luna refinement (#252) follow in a separate PR
on this runtime. No live provider, account, credential or owner data was used here.

## Versions and production feature graph

- Rig family `=0.45.0` → `=0.46.0` (published release): `rig`, `rig-agent`,
  `rig-cassette`, `rig-core`, `rig-derive`, `rig-harness-protocol`, `rig-http`,
  `rig-memory`, `rig-reqwest`, `rig-rmcp`, `rig-tungstenite`, `rig-typesafeai`. No
  other lock entry changed. Rust 1.98.1 and rmcp 3.5.0 are unchanged.
- `brn-ai` depends on `rig-memory` directly (`default-features = false`). The 0.46
  `rig` umbrella's `memory` feature pulls `rig-sqlite`, whose `links = "sqlite3"`
  conflicts with BRN's bundled rusqlite 0.40.2. BRN only uses `rig_memory::recent`.
- serde_json: 0.45's provider stack enabled `preserve_order` and `float_roundtrip`
  transitively; 0.46 no longer does. Without them, 4 provider tests failed (tool
  schema and request-body key order). `brn-ai` now enables both explicitly, so a
  test-only dependency cannot hide their absence. `cargo tree -e features` for `brn`
  and for `brn-desktop --features native-ui,native-retrieval` shows `serde_json`
  with `float_roundtrip,indexmap,preserve_order` in the production graph.
- `rig/test-utils` stays a dev and `scripted-runs` feature only.

## Behavior changes absorbed

- Rig 0.46 sends `reasoning.summary: "auto"` on ChatGPT/Copilot Responses
  requests. Assertions updated (`provider_formats_tests`, `document_refine_tests`).
  BRN neither shows nor stores reasoning summaries; no product change.
- User agent `rig/0.46.0`; test expectations updated.
- Provider stalls: Rig 0.46 ends a stream that is silent for `STREAM_IDLE_TIMEOUT`
  (300 s, wall clock) with `ProviderError::Stalled`, which reaches BRN's collector
  as `Relayed(ErrorReport { kind: Timeout })`. `error::map_provider` reports both as
  retryable origin `stalled` (only `Stalled` reports `Timeout`; tool timeouts are
  `Tool`).

## Qualification of runtime guarantees

| Guarantee | Evidence |
|---|---|
| Terra tool calls keep the same model on Responses | `copilot_terra_tool_run_retries_same_model_on_responses`. Rig's Copilot dialect sends non-`codex` IDs to Chat Completions. Copilot refuses tool calls there with 400 `unsupported_api_for_model`, mapped to `ModelRefused`, and `copilot_responses_retry` retries the same model once on Responses. BRN sets typed generation options only on Responses, so Rig's catalog refusal on Chat never fires first. |
| Stall after partial output | `stalled_stream_after_partial_output_fails_without_resend`. Partial text is not saved as the answer, the turn fails as `stalled`, and the request is sent exactly once (`ResendOnce` covers only pre-send failures and 503), so uncertain tool side effects are never repeated. |
| Stop while the stream is silent | `stop_during_silent_stream_interrupts_promptly`. Stop interrupts within 5 s, independent of the 300 s idle timer, with one request sent. |
| Final output | `real_responses_final_replaces_preview_and_is_saved_once`. |

Limitation: a provider that reasons silently for more than 300 s without stream
events is classified `stalled`. BRN keeps Rig's default; the owner sees a retryable
failure and can Continue. Live provider stalls were not observed here.

## HookTestHarness

`threads_hook_tests.rs` drives BRN's real `BatchHook` and `BudgetHook` through
`rig::test_utils::HookTestHarness`, without a model or HTTP transport:

- a rejected tool batch denies every sibling with identical no-effect feedback;
- the correction allowance is invocation-wide, then the run stops;
- forbidden calls (web search with web Off, unoffered tools, non-object arguments)
  stop before dispatch and are never corrected;
- the tool-round budget stops the turn after the limit;
- a stale scope (fence moved) or an oversized request stops the model call before
  dispatch.

Existing scripted-run tests that exercise real tool effects, the canonical store,
approvals and Undo are retained unchanged.

## Reuse pilots

### 1. Whole-request context estimate: defer

Test `rig_whole_request_estimate_pilot_is_not_an_upper_bound_for_admission`
compares `rig_memory::Prepared::whole_tokens()` (system prompt, tools, history) with
BRN's admission estimate on the real BRN preamble and tool set:

| Input | UTF-8 bytes | Rig tokens | BRN estimate (bytes) |
|---|---|---|---|
| English | 60,000 | 15,004 | 60,000 |
| Estonian | 84,000 (70,000 chars) | 21,004 | 84,000 |
| CJK | 60,000 (20,000 chars) | 15,004 | 60,000 |
| Tool-result JSON | 11,186 | 2,037 | 16,882 |
| Fixed preamble + tools | — | 9,135 | 38,118 |

- Rig's heuristic counter (4 bytes per token plus 4 per message) is an average, not
  an upper bound: it allows fewer tokens than CJK characters and ignores JSON
  escaping of tool results. BRN's UTF-8/JSON byte count bounds any byte-level BPE
  count and remains the admission rule.
- Rig's catalog gives `chatgpt/gpt-6.1-sol` an input limit of 922,000: OpenAI API
  values copied into the `chatgpt` and `copilot` entries. BRN's sourced
  subscription window is 272,000.
- Could replace: a token-unit display and anchored usage (`next_request_tokens`).
- BRN still owns: the admission upper bound, sourced route windows, result
  admission and the history cut (already `rig_memory::recent::view` with BRN's
  byte counter).
- Revisit with an optional calibrated counter once Copilot `max_prompt_tokens`
  discovery is real (#252).

### 2. rig-cassette multistep replay: defer

[Pilot source](cassette-pilot-test.rs), not compiled in this PR. It recorded BRN's
production Copilot Responses runner through `RecordVia::Direct` with explicit
`CassetteMode::Record` from a scripted in-process transport (upstream set to a
discard port and never contacted), then replayed a fresh run with explicit
`CassetteMode::Replay` and `.exact_matched()`:

- Two-turn run (`retain_progress` tool call, then final answer): replay completed,
  the tool effect landed only in the run's temporary store, and `try_finish`
  confirmed every interaction was consumed. 3 of 3 repeated runs passed.
- A changed request (effort High) was refused before any tool effect and reported.
- The recording contained no credential. Zero live provider calls; zero writes
  outside temporary directories (rig-cassette's attempt ledger goes under
  `target/`).

Finding: exact replay first failed with `ReplayMismatch`. The only difference
between two identical runs was the store-generated Chat ID in `prompt_cache_key`
and the "Current Chat" context line; tool results were deterministic. Exact replay
passed only after BRN supplied identical store identities (copies of one pristine
synthetic store). rig-cassette has no request normalizer, and `shape_matched`
erases tool arguments and content, which is too weak for BRN's regressions.

- Could replace: hand-built SSE bodies and request assertions in
  `provider_formats_tests` for multistep wire regressions.
- BRN still owns: deterministic store identities, the scripted upstream (live
  recording is outside CI authority), and canonical store, approval, provenance and
  Undo assertions.
- Deferred because recordings would come from the same scripted transport that
  `SequencedHttpClient` already provides, every packaged-guidance edit would
  invalidate each ~40 KB recorded request, and the `http` feature adds deprecated
  `serde_yaml` 0.9 and `unsafe-libyaml`. Revisit if BRN adopts recorded
  live-provider fixtures under explicit authority.

## Architecture survey

See [survey](survey.md).

## Checks

Commands follow [verification](../../../development/verification.md). macOS arm64,
Rust 1.98.1.

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| Workspace Clippy and all CI feature Clippy lanes (`-D warnings`) | PASS |
| Workspace tests (`--no-fail-fast`) | 1,513 passed, 0 failed, 9 ignored |
| `brn-ai --features capability-spike --all-targets` | 592 passed, 0 failed, 1 ignored |
| `brn` scripted document route | 1 passed |
| `brn-writing-ui` native-ui, native-test-support | 63 passed |
| `brn-desktop` native-ui | 99 passed |
| `brn-desktop` native-test-support `threads_ui::` | 447 passed, 1 ignored |
| `brn-retrieval` native (lib, model_download) | 38 passed |
| `brn-threads-app` native-retrieval | 342 passed, 3 ignored |
| `brn-desktop` native-ui, native-retrieval | 99 passed |

On macOS the workspace run needs a `TMPDIR` without a symlinked prefix; with the
default `/var/folders` path one `int_d` test refuses the symlinked data directory
(`SymlinkRefused`), as on `main`. The scripted CLI route assertion now expects
`reasoning.summary: "auto"`.

## Fallback

No blocker required the verified-0.45 fallback.
