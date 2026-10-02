# Rig spike findings

Date: 2026-10-02. Baseline: branch `main`, HEAD `7c8b392`; implementation commits:
`52b9756`, `065f454`, `745024d`; plan reconciliation: `7c8b392`.
This findings commit is `docs: record Rig spike findings`.
Environment: macOS 26.5 (25F71), Rust 1.98.1.

Live observations were supplied by the controller; Task 5 made no live calls.
Offline results below distinguish earlier Tasks 1–2 from fresh feature checks.
This is a throwaway experiment, not product implementation or native UI acceptance.

## Dependency graph

- Tasks 1–2: build, strict Clippy and four synthetic tests passed after the TLS
  feature correction. The original facade `rustls` graph did not resolve:
  weak optional dependencies pulled rig-fastembed 0.43.0 → fastembed 4.5 →
  ort rc.9, conflicting with direct fastembed 7.1 → ort rc.13.
- Compiled graph: facade `agent`/`derive`/`reqwest`, direct rig-core and
  rig-reqwest `rustls`; one ort rc.13, no rig-fastembed.
- `cargo tree -i libsqlite3-sys`: one libsqlite3-sys 0.38.2 via
  rusqlite 0.40.2. Notable duplicates from `cargo tree -d --depth 0`:
  reqwest 0.12.28/0.13.5, base64 0.13.1/0.22.1/0.23.1,
  syn 2.0.119/3.0.6, thiserror 1.0.69/2.0.21 and objc2 0.5.2/0.6.4.
- FTS5 output: `sqlite 3.53.2 fts5 hits for red: 1` (exact token, not prefix).
- Empty-cache chat rejected locally for both providers with reconnect needed,
  without device login. In-repository credentials, including through a
  symlinked ancestor, were rejected before authentication.

## API differences from the plan

- No Rig 0.43.0 signature adaptations were needed: authenticators,
  client `authenticate`, `rig::rig_reqwest::shared()`, `Tool` and agent
  streaming compiled as planned. Removed an unused `rig::prelude::*` import.
- Fix round 1 used explicit `MultiTurnStreamItem` matching rather than the
  stream-printing helper: assistant text events, tool-call markers and the
  required final response; the first stream error terminates the command.
  Cancellation drops the stream and reports whether partial text arrived.
- Tool parameter schemas use integers; addition checks signed overflow.
  These are behavior hardening, not upstream signature differences.
- The dependency manifest differs from spec section 7's facade TLS advice;
  direct TLS features are required in this pinned graph.

## ChatGPT

- Login: first code expired (auth timeout); second login succeeded.
- Account id available: yes. Email claim available in cached `id_token`: yes;
  use that claim for the connected account name. No identity values recorded.
- Cache: `chatgpt.json`, mode 0644, inside the mode-0700 credentials folder.
- Model listing failed: HTTP 400 `invalid_request_error`, missing
  `client_version`. No model list was obtained.
- Rig constants gpt-5.4 and gpt-5.3-codex were rejected for subscriptions
  (HTTP 400, model-not-supported category).
- gpt-5.5, gpt-5.6-sol, gpt-5.6-terra, gpt-6-sol and gpt-6.1-sol passed model
  routing but returned HTTP 429 `usage_limit_reached`; no completed answer.
  The response carries `resets_in_seconds`; observed reset:
  **2026-10-03 20:24 EEST**.
- Streamed tool call, restart credential reuse and cancellation: **blocked**
  by the account limit, not yet observed. Partial text before cancel and a
  successful cancel delay are therefore unqualified.

## Copilot

- Login: first code expired (auth timeout); second login succeeded.
- Account name lookup: GitHub `/user` with the saved token succeeded.
  Account-id/email-claim availability was not observed; neither is needed
  for this name lookup. No account name or identity value recorded.
- Caches: `github-token` and `copilot.json`, both mode 0644, inside the
  mode-0700 credentials folder.
- Model listing succeeded: 47 models, including gpt-4o, gpt-4.1,
  gpt-5.3-codex, gpt-5.4, gpt-5.5, claude-sonnet-5, claude-opus-5,
  gemini-3.x and text-embedding-3-small.
- gpt-4o chat-completions: streamed text, `add(2,3)` tool call, answer
  mentioning 5 and usage all observed. Separate command processes confirmed
  credential reuse after restart without login.
- Tool calls also succeeded on gpt-5.4 (Responses route) and claude-sonnet-5.
- Cancel at 1500 ms: during tool round, partial text no, exit in 1.58 s.
  Cancel at 4000 ms with a long answer: partial text yes, stream dropped,
  exit in 4.07 s. Only local cancellation observed; server stop/billing unknown.
- Session refresh: deleting only `copilot.json` then chatting succeeded
  without device login; cache recreated with mode 0644.

## Test support

Fresh Task 5 checks, both exit 0:

```sh
cargo check --manifest-path experiments/rig-spike/Cargo.toml --locked --features cassette
cargo check --manifest-path experiments/rig-spike/Cargo.toml --locked --features test-utils
```

No unlocked retry or lockfile change was needed. Both checks emitted the
upstream `block v0.1.6` future-incompatibility notice. Compilation alone does
not verify provider replay fixtures.

Replay configuration, from rig-cassette 0.43.0's registry README and HTTP API:

- `RIG_PROVIDER_TEST_MODE` defaults to `replay`; `record` contacts upstream
  and writes scrubbed fixtures. `start_at` permits explicit
  `CassetteMode::Replay` regardless of the environment. Default tests must
  explicitly prohibit recording and use synthetic fixtures, not live caches.
- HTTP fixtures are YAML at `<fixture-root>/<provider>/<scenario>.yaml`;
  pass a caller-owned root to `ProviderCassette::start`. Upstream fixture
  corpora are excluded from the published crate, so downstream tests supply
  their own fixtures. Effect logs are separate `.effects.json` files.
- HTTP replay requires direct `rig-cassette` with its `http` feature and
  `rig_cassette::http` APIs. The spike's facade `cassette` feature enables the
  agent adapter but **not** the native HTTP engine; that engine was not checked.
- A test starts a `ProviderCassette` with root, provider, `CassetteSpec` and
  upstream base URL, points its configurable provider client at
  `cassette.base_url()`, and uses dummy authentication in replay
  (`cassette.api_key` supplies a dummy key). Verify both subscription clients'
  endpoint/auth wiring separately in step 4; it was not exercised here.
- `CassetteSpec::new` enforces interaction order; `.unordered()` relaxes it.
  Await `finish()` to assert complete consumption and shut down the server;
  unconsumed/refused replay interactions cannot silently pass.
- Agent-level effect replay instead uses `EffectLogRecorder`,
  `AgentBuilder::record_to`, `AgentReplayExt::stamp` and
  `agent::replay::register_all`; `keeping_stream_events()` retains stream
  boundaries needed for partial-output/error-order assertions.

## Decision

**Copilot: GO. ChatGPT: CONDITIONAL GO** — login, account identity and model
routing work; streamed tool call, restart reuse and cancel are not yet observed
because the account hit its usage limit. Rerun Task 4 for ChatGPT after the quota
resets (**2026-10-03 20:24 EEST**) with a supported model (e.g. gpt-5.5) before
step 4's ChatGPT work is accepted.

### Required changes for step 4 / spec

- Do not enable Rig facade `rustls`/`native-tls` with fastembed 7; use direct
  rig-core/rig-reqwest TLS features instead.
- Maintain a fixed ChatGPT model list: `list_models` fails and Rig's
  gpt-5.4/gpt-5.3-codex constants are rejected for subscriptions.
- Rig writes credential caches mode 0644: create the credentials folder 0700,
  chmod cache files to 0600 after login/refresh, and check permissions at start.
- Device-code login can time out quickly: show a clear
  "code expired, try again" path.
- Rig error text contains raw response bodies: map errors to auth/reconnect,
  rate/usage limit with reset time, model not supported, network or other
  categories before displaying or logging them.
- Usage-limit HTTP 429 carries `resets_in_seconds`; show the reset to the user.
- Obtain Copilot's account name from GitHub `/user` with the saved token,
  and ChatGPT's name from the `id_token` email claim.
- `block v0.1.6` future-incompatibility via GPUI is upstream; no action now.
- For offline HTTP replay, enable/check rig-cassette's direct `http` feature,
  provide sanitized synthetic fixtures for both provider formats, and verify
  subscription endpoint/auth wiring; the successful facade check is not this
  acceptance test.
